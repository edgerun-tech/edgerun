use lifegraph_biometrics::{BiometricModality, BiometricState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraReaderInfo {
    pub provider: String,
    pub reader_name: String,
    pub supports_face_detection: bool,
    pub supports_face_matching: bool,
    pub supports_liveness_detection: bool,
    pub hardware_protected_match: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraPixelFormat {
    Mjpeg,
    Yuyv,
    Nv12,
    Rgb24,
    Gray8,
    Other(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: CameraPixelFormat,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraStreamRole {
    Unknown,
    Rgb,
    Infrared,
    Depth,
    Monochrome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedCameraFrame {
    pub rgb: Option<CameraFrame>,
    pub infrared: Option<CameraFrame>,
    pub depth: Option<CameraFrame>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraBiometricPurpose {
    Presence,
    Enrollment,
    Verification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CameraCaptureQuality {
    Poor,
    Fair,
    Good,
    Excellent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraCapture {
    pub frame: CameraFrame,
    pub quality: CameraCaptureQuality,
    pub face_bounds: Option<FaceBounds>,
    pub state: BiometricState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraLivenessChallengeKind {
    PassivePresence,
    Blink,
    TurnLeft,
    TurnRight,
    LookUp,
    LookDown,
    MoveCloser,
    MoveFarther,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraLivenessChallenge {
    pub kind: CameraLivenessChallengeKind,
    pub timeout_ms: u32,
    pub require_rgb: bool,
    pub require_infrared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraLivenessFrameObservation {
    pub role: CameraStreamRole,
    pub face_detected: bool,
    pub quality: CameraCaptureQuality,
    pub face_bounds: Option<FaceBounds>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraLivenessEvidence {
    pub challenge: CameraLivenessChallenge,
    pub observations: Vec<CameraLivenessFrameObservation>,
    pub paired_capture: bool,
    pub motion_detected: bool,
    pub blink_detected: bool,
    pub face_present_in_rgb: bool,
    pub face_present_in_infrared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraLivenessResult {
    pub passed: bool,
    pub evidence: CameraLivenessEvidence,
    pub state: BiometricState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraEnrollRequest {
    pub label: String,
    pub samples_required: u8,
    pub require_liveness: bool,
    pub require_hardware_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraEnrollmentSession {
    pub session_id: String,
    pub label: String,
    pub samples_required: u8,
    pub samples_collected: u8,
    pub require_liveness: bool,
    pub require_hardware_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraEnrollProgress {
    pub session: CameraEnrollmentSession,
    pub complete: bool,
    pub template_id: Option<String>,
    pub last_quality: CameraCaptureQuality,
    pub face_detected: bool,
    pub liveness_detected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraTemplateRecord {
    pub template_id: String,
    pub label: String,
    pub enrolled_at_unix_ms: i64,
    pub last_verified_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraVerifyRequest {
    pub allowed_template_ids: Vec<String>,
    pub require_liveness: bool,
    pub require_hardware_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraVerification {
    pub matched: bool,
    pub template_id: Option<String>,
    pub liveness_detected: bool,
    pub face_detected: bool,
    pub state: BiometricState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraBiometricError {
    Provider(String),
    InvalidRequest(&'static str),
    InvalidState(&'static str),
    UnsupportedOperation(&'static str),
}

impl core::fmt::Display for CameraBiometricError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Provider(msg) => f.write_str(msg),
            Self::InvalidRequest(msg) => write!(f, "invalid camera biometric request: {msg}"),
            Self::InvalidState(msg) => write!(f, "invalid camera biometric state: {msg}"),
            Self::UnsupportedOperation(msg) => {
                write!(f, "unsupported camera biometric operation: {msg}")
            }
        }
    }
}

impl std::error::Error for CameraBiometricError {}

pub trait CameraBiometricReader {
    fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError>;
    fn capture(
        &mut self,
        purpose: CameraBiometricPurpose,
        timeout_ms: u32,
    ) -> Result<CameraCapture, CameraBiometricError>;
    fn begin_enrollment(
        &mut self,
        request: &CameraEnrollRequest,
    ) -> Result<CameraEnrollmentSession, CameraBiometricError>;
    fn enroll_step(
        &mut self,
        session_id: &str,
        capture: &CameraCapture,
    ) -> Result<CameraEnrollProgress, CameraBiometricError>;
    fn finish_enrollment(
        &mut self,
        session_id: &str,
    ) -> Result<CameraTemplateRecord, CameraBiometricError>;
    fn verify_capture(
        &mut self,
        request: &CameraVerifyRequest,
        capture: &CameraCapture,
    ) -> Result<CameraVerification, CameraBiometricError>;
    fn list_templates(&self) -> Result<Vec<CameraTemplateRecord>, CameraBiometricError>;
    fn delete_template(&mut self, template_id: &str) -> Result<(), CameraBiometricError>;
}

pub trait PairedCameraBiometricReader: CameraBiometricReader {
    fn supported_stream_roles(&self) -> Result<Vec<CameraStreamRole>, CameraBiometricError>;
    fn capture_paired(
        &mut self,
        purpose: CameraBiometricPurpose,
        timeout_ms: u32,
    ) -> Result<PairedCameraFrame, CameraBiometricError>;
    fn evaluate_liveness(
        &mut self,
        challenge: &CameraLivenessChallenge,
        capture: &PairedCameraFrame,
    ) -> Result<CameraLivenessResult, CameraBiometricError>;
}

pub fn default_face_biometric_state(
    verified: bool,
    hardware_protected: bool,
    user_present: bool,
) -> BiometricState {
    BiometricState {
        modality: Some(BiometricModality::Face),
        verified,
        hardware_protected,
        user_present,
    }
}

pub fn validate_camera_enroll_request(
    request: &CameraEnrollRequest,
) -> Result<(), CameraBiometricError> {
    if request.label.trim().is_empty() {
        return Err(CameraBiometricError::InvalidRequest(
            "enrollment label must not be empty",
        ));
    }
    if request.samples_required == 0 {
        return Err(CameraBiometricError::InvalidRequest(
            "samples_required must be greater than zero",
        ));
    }
    Ok(())
}

pub fn default_liveness_challenge() -> CameraLivenessChallenge {
    CameraLivenessChallenge {
        kind: CameraLivenessChallengeKind::PassivePresence,
        timeout_ms: 1_500,
        require_rgb: true,
        require_infrared: false,
    }
}

pub fn validate_liveness_challenge(
    challenge: &CameraLivenessChallenge,
) -> Result<(), CameraBiometricError> {
    if challenge.timeout_ms == 0 {
        return Err(CameraBiometricError::InvalidRequest(
            "liveness timeout must be greater than zero",
        ));
    }
    if !challenge.require_rgb && !challenge.require_infrared {
        return Err(CameraBiometricError::InvalidRequest(
            "liveness challenge must require at least one stream",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_biometrics::BiometricAssuranceStrength;

    #[test]
    fn default_state_is_face_modality() {
        let state = default_face_biometric_state(true, false, true);
        assert_eq!(state.modality, Some(BiometricModality::Face));
        assert!(state.satisfies(BiometricAssuranceStrength::BiometricMatch));
    }

    #[test]
    fn enroll_request_must_have_label_and_samples() {
        let err = validate_camera_enroll_request(&CameraEnrollRequest {
            label: " ".into(),
            samples_required: 0,
            require_liveness: true,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CameraBiometricError::InvalidRequest("enrollment label must not be empty")
        );
    }

    #[test]
    fn default_liveness_challenge_requires_rgb() {
        let challenge = default_liveness_challenge();
        assert_eq!(challenge.kind, CameraLivenessChallengeKind::PassivePresence);
        assert!(challenge.require_rgb);
        assert!(!challenge.require_infrared);
    }

    #[test]
    fn liveness_challenge_requires_nonzero_timeout_and_stream() {
        let err = validate_liveness_challenge(&CameraLivenessChallenge {
            kind: CameraLivenessChallengeKind::Blink,
            timeout_ms: 0,
            require_rgb: false,
            require_infrared: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CameraBiometricError::InvalidRequest("liveness timeout must be greater than zero")
        );
    }
}
