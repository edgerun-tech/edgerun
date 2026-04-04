use lifegraph_biometrics::{BiometricModality, BiometricState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintReaderInfo {
    pub provider: String,
    pub reader_name: String,
    pub supports_match_on_sensor: bool,
    pub supports_persistent_templates: bool,
    pub hardware_protected_match: bool,
    pub max_templates: Option<u32>,
    pub usb_vendor_id: Option<u16>,
    pub usb_product_id: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FingerprintCapturePurpose {
    Enrollment,
    Verification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FingerprintCaptureQuality {
    Poor,
    Fair,
    Good,
    Excellent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintCapture {
    pub bytes: Vec<u8>,
    pub quality: FingerprintCaptureQuality,
    pub state: BiometricState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintEnrollRequest {
    pub label: String,
    pub samples_required: u8,
    pub require_hardware_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintEnrollmentSession {
    pub session_id: String,
    pub label: String,
    pub samples_required: u8,
    pub samples_collected: u8,
    pub require_hardware_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintEnrollProgress {
    pub session: FingerprintEnrollmentSession,
    pub complete: bool,
    pub template_id: Option<String>,
    pub last_quality: FingerprintCaptureQuality,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintTemplateRecord {
    pub template_id: String,
    pub label: String,
    pub enrolled_at_unix_ms: i64,
    pub last_verified_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintVerifyRequest {
    pub allowed_template_ids: Vec<String>,
    pub require_hardware_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintVerification {
    pub matched: bool,
    pub template_id: Option<String>,
    pub state: BiometricState,
    pub false_accept_rate_per_million: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FingerprintError {
    Provider(String),
    InvalidRequest(&'static str),
    InvalidState(&'static str),
    UnsupportedOperation(&'static str),
}

impl core::fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Provider(msg) => f.write_str(msg),
            Self::InvalidRequest(msg) => write!(f, "invalid fingerprint request: {msg}"),
            Self::InvalidState(msg) => write!(f, "invalid fingerprint state: {msg}"),
            Self::UnsupportedOperation(msg) => {
                write!(f, "unsupported fingerprint operation: {msg}")
            }
        }
    }
}

impl std::error::Error for FingerprintError {}

pub trait FingerprintReader {
    fn reader_info(&self) -> Result<FingerprintReaderInfo, FingerprintError>;
    fn list_templates(&self) -> Result<Vec<FingerprintTemplateRecord>, FingerprintError>;
    fn capture(
        &mut self,
        purpose: FingerprintCapturePurpose,
        timeout_ms: u32,
    ) -> Result<FingerprintCapture, FingerprintError>;
    fn begin_enrollment(
        &mut self,
        request: &FingerprintEnrollRequest,
    ) -> Result<FingerprintEnrollmentSession, FingerprintError>;
    fn enroll_step(
        &mut self,
        session_id: &str,
        capture: &FingerprintCapture,
    ) -> Result<FingerprintEnrollProgress, FingerprintError>;
    fn finish_enrollment(
        &mut self,
        session_id: &str,
    ) -> Result<FingerprintTemplateRecord, FingerprintError>;
    fn verify_capture(
        &mut self,
        request: &FingerprintVerifyRequest,
        capture: &FingerprintCapture,
    ) -> Result<FingerprintVerification, FingerprintError>;
    fn delete_template(&mut self, template_id: &str) -> Result<(), FingerprintError>;
}

pub fn default_fingerprint_biometric_state(
    verified: bool,
    hardware_protected: bool,
    user_present: bool,
) -> BiometricState {
    BiometricState {
        modality: Some(BiometricModality::Fingerprint),
        verified,
        hardware_protected,
        user_present,
    }
}

pub fn validate_enroll_request(request: &FingerprintEnrollRequest) -> Result<(), FingerprintError> {
    if request.label.trim().is_empty() {
        return Err(FingerprintError::InvalidRequest(
            "enrollment label must not be empty",
        ));
    }
    if request.samples_required == 0 {
        return Err(FingerprintError::InvalidRequest(
            "samples_required must be greater than zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_biometrics::BiometricAssuranceStrength;

    #[test]
    fn default_state_is_fingerprint_modality() {
        let state = default_fingerprint_biometric_state(true, false, true);
        assert_eq!(state.modality, Some(BiometricModality::Fingerprint));
        assert!(state.satisfies(BiometricAssuranceStrength::BiometricMatch));
    }

    #[test]
    fn enroll_request_must_have_label_and_samples() {
        let err = validate_enroll_request(&FingerprintEnrollRequest {
            label: " ".into(),
            samples_required: 0,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            FingerprintError::InvalidRequest("enrollment label must not be empty")
        );

        let err = validate_enroll_request(&FingerprintEnrollRequest {
            label: "primary finger".into(),
            samples_required: 0,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            FingerprintError::InvalidRequest("samples_required must be greater than zero")
        );
    }
}
