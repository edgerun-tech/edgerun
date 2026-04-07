use edgerun_biometrics::{BiometricModality, BiometricState};

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
    use edgerun_biometrics::BiometricAssuranceStrength;

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

    #[test]
    fn validate_enroll_request_with_valid_input() {
        let result = validate_enroll_request(&FingerprintEnrollRequest {
            label: "Right Index".into(),
            samples_required: 3,
            require_hardware_match: true,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn validate_enroll_request_rejects_empty_label() {
        let err = validate_enroll_request(&FingerprintEnrollRequest {
            label: "".into(),
            samples_required: 1,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert!(matches!(err, FingerprintError::InvalidRequest(_)));
    }

    #[test]
    fn validate_enroll_request_rejects_whitespace_only_label() {
        let err = validate_enroll_request(&FingerprintEnrollRequest {
            label: "   \t\n".into(),
            samples_required: 1,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert!(matches!(err, FingerprintError::InvalidRequest(_)));
    }

    #[test]
    fn fingerprint_error_display() {
        let err = FingerprintError::InvalidRequest("test reason");
        let msg = format!("{err}");
        assert!(msg.contains("invalid fingerprint request"));
        assert!(msg.contains("test reason"));

        let err = FingerprintError::InvalidState("bad state");
        let msg = format!("{err}");
        assert!(msg.contains("invalid fingerprint state"));

        let err = FingerprintError::UnsupportedOperation("not supported");
        let msg = format!("{err}");
        assert!(msg.contains("unsupported fingerprint operation"));

        let err = FingerprintError::Provider("provider error".into());
        let msg = format!("{err}");
        assert_eq!(msg, "provider error");
    }

    #[test]
    fn fingerprint_error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(FingerprintError::InvalidRequest("test"));
        assert!(err.to_string().contains("invalid fingerprint request"));
    }

    #[test]
    fn fingerprint_capture_quality_ordering() {
        assert!(FingerprintCaptureQuality::Poor < FingerprintCaptureQuality::Fair);
        assert!(FingerprintCaptureQuality::Fair < FingerprintCaptureQuality::Good);
        assert!(FingerprintCaptureQuality::Good < FingerprintCaptureQuality::Excellent);
    }

    #[test]
    fn fingerprint_capture_purpose_variants() {
        assert_ne!(
            FingerprintCapturePurpose::Enrollment,
            FingerprintCapturePurpose::Verification
        );
    }

    #[test]
    fn fingerprint_reader_info_clone_debug() {
        let info = FingerprintReaderInfo {
            provider: "test-provider".into(),
            reader_name: "Test Reader".into(),
            supports_match_on_sensor: true,
            supports_persistent_templates: false,
            hardware_protected_match: true,
            max_templates: Some(10),
            usb_vendor_id: Some(0x1234),
            usb_product_id: Some(0x5678),
        };
        let cloned = info.clone();
        assert_eq!(info.provider, cloned.provider);
        assert_eq!(info.max_templates, cloned.max_templates);
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("FingerprintReaderInfo"));
    }

    #[test]
    fn fingerprint_enrollment_session_clone_debug() {
        let session = FingerprintEnrollmentSession {
            session_id: "sess-001".into(),
            label: "Right Thumb".into(),
            samples_required: 5,
            samples_collected: 2,
            require_hardware_match: true,
        };
        let cloned = session.clone();
        assert_eq!(session.session_id, cloned.session_id);
        assert_eq!(session.samples_collected, cloned.samples_collected);
    }

    #[test]
    fn fingerprint_enroll_progress_clone_debug() {
        let progress = FingerprintEnrollProgress {
            session: FingerprintEnrollmentSession {
                session_id: "sess-001".into(),
                label: "Left Index".into(),
                samples_required: 3,
                samples_collected: 1,
                require_hardware_match: false,
            },
            complete: false,
            template_id: None,
            last_quality: FingerprintCaptureQuality::Good,
        };
        let cloned = progress.clone();
        assert!(!cloned.complete);
        assert_eq!(cloned.last_quality, FingerprintCaptureQuality::Good);
    }

    #[test]
    fn fingerprint_template_record_clone_debug() {
        let record = FingerprintTemplateRecord {
            template_id: "tpl-001".into(),
            label: "Right Index".into(),
            enrolled_at_unix_ms: 1_000_000,
            last_verified_unix_ms: Some(2_000_000),
        };
        let cloned = record.clone();
        assert_eq!(record.template_id, cloned.template_id);
        assert_eq!(record.last_verified_unix_ms, cloned.last_verified_unix_ms);
    }

    #[test]
    fn fingerprint_verify_request_clone_debug() {
        let request = FingerprintVerifyRequest {
            allowed_template_ids: vec!["tpl-001".into(), "tpl-002".into()],
            require_hardware_match: true,
        };
        let cloned = request.clone();
        assert_eq!(request.allowed_template_ids, cloned.allowed_template_ids);
    }

    #[test]
    fn fingerprint_verification_clone_debug() {
        let verification = FingerprintVerification {
            matched: true,
            template_id: Some("tpl-001".into()),
            state: default_fingerprint_biometric_state(true, true, true),
            false_accept_rate_per_million: Some(100),
        };
        let cloned = verification.clone();
        assert!(cloned.matched);
        assert_eq!(cloned.false_accept_rate_per_million, Some(100));
    }

    #[test]
    fn fingerprint_capture_clone_debug() {
        let capture = FingerprintCapture {
            bytes: vec![1, 2, 3],
            quality: FingerprintCaptureQuality::Excellent,
            state: default_fingerprint_biometric_state(false, false, true),
        };
        let cloned = capture.clone();
        assert_eq!(capture.bytes, cloned.bytes);
        assert_eq!(capture.quality, cloned.quality);
    }

    #[test]
    fn default_fingerprint_biometric_state_values() {
        let state = default_fingerprint_biometric_state(true, true, true);
        assert!(state.verified);
        assert!(state.hardware_protected);
        assert!(state.user_present);
        assert_eq!(state.modality, Some(BiometricModality::Fingerprint));

        let state = default_fingerprint_biometric_state(false, false, false);
        assert!(!state.verified);
        assert!(!state.hardware_protected);
        assert!(!state.user_present);
    }
}
