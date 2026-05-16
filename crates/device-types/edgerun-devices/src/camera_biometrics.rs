extern crate alloc;
mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

use crate::biometrics::{BiometricModality, BiometricState};
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole, capability_descriptor,
};
use prelude::v1::*;

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

impl core::error::Error for CameraBiometricError {}

pub trait CameraBiometricReader: CapabilityProvider {
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

pub fn default_camera_descriptor(
    provider: &str,
    instance_id: &str,
    supports_biometric: bool,
) -> CapabilityDescriptor {
    let mut modalities = vec![CapabilityModality::Visual];
    if supports_biometric {
        modalities.push(CapabilityModality::Biometric);
    }
    let mut event_kinds = vec![CapabilityEventKind::Visual];
    if supports_biometric {
        event_kinds.push(CapabilityEventKind::Biometric);
    }
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Input,
        &modalities,
        &event_kinds,
        &[CapabilityOperation::Query, CapabilityOperation::Capture],
        Vec::new(),
    )
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
    use crate::biometrics::BiometricAssuranceStrength;

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
    fn default_camera_descriptor_declares_visual_input() {
        let descriptor = default_camera_descriptor("v4l2-camera", "/dev/video0", false);
        assert_eq!(descriptor.provider_name, "v4l2-camera");
        assert_eq!(descriptor.provider_instance_id, "/dev/video0");
        assert_eq!(descriptor.role, CapabilityRole::Input as i32);
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Visual as i32]
        );
        assert_eq!(
            descriptor.event_kinds,
            vec![CapabilityEventKind::Visual as i32]
        );
        assert_eq!(
            descriptor.operations,
            vec![
                CapabilityOperation::Query as i32,
                CapabilityOperation::Capture as i32
            ]
        );
    }

    #[test]
    fn default_camera_descriptor_can_declare_biometric_input() {
        let descriptor = default_camera_descriptor("v4l2-camera", "rgb-ir", true);
        assert_eq!(
            descriptor.modalities,
            vec![
                CapabilityModality::Visual as i32,
                CapabilityModality::Biometric as i32
            ]
        );
        assert_eq!(
            descriptor.event_kinds,
            vec![
                CapabilityEventKind::Visual as i32,
                CapabilityEventKind::Biometric as i32
            ]
        );
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

    #[test]
    fn validate_camera_enroll_request_with_valid_input() {
        let result = validate_camera_enroll_request(&CameraEnrollRequest {
            label: "Front Camera".into(),
            samples_required: 5,
            require_liveness: true,
            require_hardware_match: false,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn validate_camera_enroll_request_rejects_empty_label() {
        let err = validate_camera_enroll_request(&CameraEnrollRequest {
            label: "".into(),
            samples_required: 1,
            require_liveness: false,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert!(matches!(err, CameraBiometricError::InvalidRequest(_)));
    }

    #[test]
    fn validate_camera_enroll_request_rejects_zero_samples() {
        let err = validate_camera_enroll_request(&CameraEnrollRequest {
            label: "Valid Label".into(),
            samples_required: 0,
            require_liveness: false,
            require_hardware_match: false,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            CameraBiometricError::InvalidRequest("samples_required must be greater than zero")
        ));
    }

    #[test]
    fn validate_liveness_challenge_rejects_no_streams() {
        let err = validate_liveness_challenge(&CameraLivenessChallenge {
            kind: CameraLivenessChallengeKind::PassivePresence,
            timeout_ms: 1000,
            require_rgb: false,
            require_infrared: false,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            CameraBiometricError::InvalidRequest(
                "liveness challenge must require at least one stream"
            )
        ));
    }

    #[test]
    fn validate_liveness_challenge_accepts_infrared_only() {
        let result = validate_liveness_challenge(&CameraLivenessChallenge {
            kind: CameraLivenessChallengeKind::PassivePresence,
            timeout_ms: 1500,
            require_rgb: false,
            require_infrared: true,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn validate_liveness_challenge_accepts_rgb_only() {
        let result = validate_liveness_challenge(&CameraLivenessChallenge {
            kind: CameraLivenessChallengeKind::Blink,
            timeout_ms: 2000,
            require_rgb: true,
            require_infrared: false,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn camera_biometric_error_display() {
        let err = CameraBiometricError::InvalidRequest("test");
        let msg = format!("{err}");
        assert!(msg.contains("invalid camera biometric request"));

        let err = CameraBiometricError::InvalidState("bad state");
        let msg = format!("{err}");
        assert!(msg.contains("invalid camera biometric state"));

        let err = CameraBiometricError::UnsupportedOperation("not supported");
        let msg = format!("{err}");
        assert!(msg.contains("unsupported camera biometric operation"));

        let err = CameraBiometricError::Provider("provider error".into());
        assert_eq!(format!("{err}"), "provider error");
    }

    #[test]
    fn camera_biometric_error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(CameraBiometricError::InvalidRequest("test"));
        assert!(err.to_string().contains("invalid camera biometric request"));
    }

    #[test]
    fn camera_capture_quality_ordering() {
        assert!(CameraCaptureQuality::Poor < CameraCaptureQuality::Fair);
        assert!(CameraCaptureQuality::Fair < CameraCaptureQuality::Good);
        assert!(CameraCaptureQuality::Good < CameraCaptureQuality::Excellent);
    }

    #[test]
    fn camera_pixel_format_variants() {
        assert_ne!(CameraPixelFormat::Mjpeg, CameraPixelFormat::Yuyv);
        assert_ne!(CameraPixelFormat::Nv12, CameraPixelFormat::Rgb24);
        assert_ne!(
            CameraPixelFormat::Gray8,
            CameraPixelFormat::Other(0x12345678)
        );
        assert_eq!(CameraPixelFormat::Other(42), CameraPixelFormat::Other(42));
        assert_ne!(CameraPixelFormat::Other(1), CameraPixelFormat::Other(2));
    }

    #[test]
    fn camera_stream_role_variants() {
        assert_ne!(CameraStreamRole::Unknown, CameraStreamRole::Rgb);
        assert_ne!(CameraStreamRole::Rgb, CameraStreamRole::Infrared);
        assert_ne!(CameraStreamRole::Infrared, CameraStreamRole::Depth);
        assert_ne!(CameraStreamRole::Depth, CameraStreamRole::Monochrome);
    }

    #[test]
    fn camera_liveness_challenge_kind_variants() {
        let kinds = [
            CameraLivenessChallengeKind::PassivePresence,
            CameraLivenessChallengeKind::Blink,
            CameraLivenessChallengeKind::TurnLeft,
            CameraLivenessChallengeKind::TurnRight,
            CameraLivenessChallengeKind::LookUp,
            CameraLivenessChallengeKind::LookDown,
            CameraLivenessChallengeKind::MoveCloser,
            CameraLivenessChallengeKind::MoveFarther,
        ];
        // All should be distinct
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "kinds at {i} and {j} should differ");
                }
            }
        }
    }

    #[test]
    fn camera_frame_clone_debug() {
        let frame = CameraFrame {
            width: 1920,
            height: 1080,
            stride: 3840,
            format: CameraPixelFormat::Rgb24,
            bytes: vec![0; 100],
        };
        let cloned = frame.clone();
        assert_eq!(frame.width, cloned.width);
        assert_eq!(frame.height, cloned.height);
        assert_eq!(frame.bytes, cloned.bytes);
        let debug_str = format!("{frame:?}");
        assert!(debug_str.contains("CameraFrame"));
    }

    #[test]
    fn face_bounds_clone_debug() {
        let bounds = FaceBounds {
            x: 100,
            y: 200,
            width: 300,
            height: 400,
        };
        let cloned = bounds.clone();
        assert_eq!(bounds.x, cloned.x);
        assert_eq!(bounds.height, cloned.height);
    }

    #[test]
    fn camera_capture_clone_debug() {
        let capture = CameraCapture {
            frame: CameraFrame {
                width: 640,
                height: 480,
                stride: 640,
                format: CameraPixelFormat::Gray8,
                bytes: vec![],
            },
            quality: CameraCaptureQuality::Good,
            face_bounds: Some(FaceBounds {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
            }),
            state: default_face_biometric_state(false, false, true),
        };
        let cloned = capture.clone();
        assert_eq!(capture.quality, cloned.quality);
        assert!(cloned.face_bounds.is_some());
    }

    #[test]
    fn camera_enrollment_session_clone_debug() {
        let session = CameraEnrollmentSession {
            session_id: "sess-001".into(),
            label: "User Face".into(),
            samples_required: 3,
            samples_collected: 1,
            require_liveness: true,
            require_hardware_match: false,
        };
        let cloned = session.clone();
        assert_eq!(session.session_id, cloned.session_id);
        assert!(cloned.require_liveness);
    }

    #[test]
    fn camera_enroll_progress_clone_debug() {
        let progress = CameraEnrollProgress {
            session: CameraEnrollmentSession {
                session_id: "sess-001".into(),
                label: "Face".into(),
                samples_required: 1,
                samples_collected: 1,
                require_liveness: true,
                require_hardware_match: false,
            },
            complete: false,
            template_id: None,
            last_quality: CameraCaptureQuality::Good,
            face_detected: true,
            liveness_detected: false,
        };
        let cloned = progress.clone();
        assert!(cloned.face_detected);
        assert!(!cloned.liveness_detected);
    }

    #[test]
    fn camera_template_record_clone_debug() {
        let record = CameraTemplateRecord {
            template_id: "tpl-001".into(),
            label: "Face Template".into(),
            enrolled_at_unix_ms: 1_000_000,
            last_verified_unix_ms: Some(2_000_000),
        };
        let cloned = record.clone();
        assert_eq!(record.template_id, cloned.template_id);
    }

    #[test]
    fn camera_verify_request_clone_debug() {
        let request = CameraVerifyRequest {
            allowed_template_ids: vec!["tpl-1".into()],
            require_liveness: true,
            require_hardware_match: false,
        };
        let cloned = request.clone();
        assert_eq!(request.allowed_template_ids, cloned.allowed_template_ids);
        assert!(cloned.require_liveness);
    }

    #[test]
    fn camera_verification_clone_debug() {
        let verification = CameraVerification {
            matched: true,
            template_id: Some("tpl-001".into()),
            liveness_detected: true,
            face_detected: true,
            state: default_face_biometric_state(true, false, true),
        };
        let cloned = verification.clone();
        assert!(cloned.matched);
        assert!(cloned.liveness_detected);
    }

    #[test]
    fn camera_liveness_challenge_clone_debug() {
        let challenge = CameraLivenessChallenge {
            kind: CameraLivenessChallengeKind::Blink,
            timeout_ms: 2000,
            require_rgb: true,
            require_infrared: true,
        };
        let cloned = challenge.clone();
        assert_eq!(challenge.kind, cloned.kind);
        assert_eq!(challenge.timeout_ms, cloned.timeout_ms);
    }

    #[test]
    fn camera_liveness_evidence_clone_debug() {
        let evidence = CameraLivenessEvidence {
            challenge: CameraLivenessChallenge {
                kind: CameraLivenessChallengeKind::PassivePresence,
                timeout_ms: 1500,
                require_rgb: true,
                require_infrared: false,
            },
            observations: vec![],
            paired_capture: false,
            motion_detected: false,
            blink_detected: false,
            face_present_in_rgb: true,
            face_present_in_infrared: false,
        };
        let cloned = evidence.clone();
        assert!(cloned.face_present_in_rgb);
        assert!(!cloned.paired_capture);
    }

    #[test]
    fn camera_liveness_result_clone_debug() {
        let result = CameraLivenessResult {
            passed: true,
            evidence: CameraLivenessEvidence {
                challenge: CameraLivenessChallenge {
                    kind: CameraLivenessChallengeKind::PassivePresence,
                    timeout_ms: 1000,
                    require_rgb: true,
                    require_infrared: false,
                },
                observations: vec![],
                paired_capture: true,
                motion_detected: false,
                blink_detected: false,
                face_present_in_rgb: true,
                face_present_in_infrared: true,
            },
            state: default_face_biometric_state(true, false, true),
        };
        let cloned = result.clone();
        assert!(cloned.passed);
        assert!(cloned.evidence.paired_capture);
    }

    #[test]
    fn camera_reader_info_clone_debug() {
        let info = CameraReaderInfo {
            provider: "test".into(),
            reader_name: "Test Camera".into(),
            supports_face_detection: true,
            supports_face_matching: false,
            supports_liveness_detection: true,
            hardware_protected_match: false,
        };
        let cloned = info.clone();
        assert_eq!(info.reader_name, cloned.reader_name);
        assert!(cloned.supports_face_detection);
    }

    #[test]
    fn paired_camera_frame_clone_debug() {
        let frame = PairedCameraFrame {
            rgb: Some(CameraFrame {
                width: 1920,
                height: 1080,
                stride: 3840,
                format: CameraPixelFormat::Mjpeg,
                bytes: vec![],
            }),
            infrared: None,
            depth: None,
        };
        let cloned = frame.clone();
        assert!(cloned.rgb.is_some());
        assert!(cloned.infrared.is_none());
    }

    #[test]
    fn camera_liveness_frame_observation_clone_debug() {
        let obs = CameraLivenessFrameObservation {
            role: CameraStreamRole::Rgb,
            face_detected: true,
            quality: CameraCaptureQuality::Excellent,
            face_bounds: None,
        };
        let cloned = obs.clone();
        assert_eq!(obs.role, cloned.role);
        assert_eq!(obs.quality, cloned.quality);
    }

    #[test]
    fn default_face_biometric_state_values() {
        let state = default_face_biometric_state(true, true, true);
        assert_eq!(state.modality, Some(BiometricModality::Face));
        assert!(state.verified);
        assert!(state.hardware_protected);
        assert!(state.user_present);

        let state = default_face_biometric_state(false, false, false);
        assert!(!state.verified);
        assert!(!state.hardware_protected);
        assert!(!state.user_present);
    }
}
