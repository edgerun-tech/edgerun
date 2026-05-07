//! Camera biometric reader remote adapters (single-stream and paired RGB+IR+Depth).

use crate::prelude::v1::*;
use edgerun_biometrics::BiometricModality;
use edgerun_camera_biometrics::{
    CameraBiometricError, CameraBiometricPurpose, CameraBiometricReader, CameraCapture,
    CameraCaptureQuality, CameraFrame, CameraPixelFormat, FaceBounds, PairedCameraBiometricReader,
    PairedCameraFrame,
};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_core::protocol::capability::CapabilityInvocation;
use edgerun_core::protocol::capability_runtime::CapabilitySessionEvent;

use crate::adapters::common::stream_oriented_error;
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

fn map_camera_error(err: CameraBiometricError) -> CapabilityError {
    match err {
        CameraBiometricError::Provider(msg) => CapabilityError::Provider(msg),
        CameraBiometricError::InvalidRequest(msg) => CapabilityError::InvalidRequest(msg),
        CameraBiometricError::InvalidState(msg) => CapabilityError::Provider(msg.into()),
        CameraBiometricError::UnsupportedOperation(msg) => CapabilityError::Unsupported(msg),
    }
}

fn camera_pixel_format_to_u32(format: CameraPixelFormat) -> u32 {
    match format {
        CameraPixelFormat::Mjpeg => 1,
        CameraPixelFormat::Yuyv => 2,
        CameraPixelFormat::Nv12 => 3,
        CameraPixelFormat::Rgb24 => 4,
        CameraPixelFormat::Gray8 => 5,
        CameraPixelFormat::Other(v) => v | 0x8000_0000,
    }
}

fn camera_pixel_format_from_u32(raw: u32) -> Result<CameraPixelFormat, CapabilityError> {
    Ok(match raw {
        1 => CameraPixelFormat::Mjpeg,
        2 => CameraPixelFormat::Yuyv,
        3 => CameraPixelFormat::Nv12,
        4 => CameraPixelFormat::Rgb24,
        5 => CameraPixelFormat::Gray8,
        other if other & 0x8000_0000 != 0 => CameraPixelFormat::Other(other & 0x7fff_ffff),
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote camera pixel format is unknown",
            ));
        }
    })
}

fn camera_capture_quality_to_u8(q: CameraCaptureQuality) -> u8 {
    match q {
        CameraCaptureQuality::Poor => 1,
        CameraCaptureQuality::Fair => 2,
        CameraCaptureQuality::Good => 3,
        CameraCaptureQuality::Excellent => 4,
    }
}

fn camera_capture_quality_from_u8(v: u8) -> Result<CameraCaptureQuality, CapabilityError> {
    Ok(match v {
        1 => CameraCaptureQuality::Poor,
        2 => CameraCaptureQuality::Fair,
        3 => CameraCaptureQuality::Good,
        4 => CameraCaptureQuality::Excellent,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote camera capture quality is invalid",
            ));
        }
    })
}

// --- Public encode/decode ---

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct CameraFrameWire {
    width: u32,
    height: u32,
    stride: u32,
    format: u32,
    bytes: Vec<u8>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct FaceBoundsWire {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct BiometricStateWire {
    modality: u8,
    verified: bool,
    hardware_protected: bool,
    user_present: bool,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct CameraCaptureWire {
    frame: CameraFrameWire,
    quality: u8,
    face_bounds: Option<FaceBoundsWire>,
    state: BiometricStateWire,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    edgerun_wire::Archive,
    edgerun_wire::Serialize,
    edgerun_wire::Deserialize,
)]
#[rkyv(crate = edgerun_wire)]
struct PairedCameraFrameWire {
    rgb: Option<CameraFrameWire>,
    infrared: Option<CameraFrameWire>,
    depth: Option<CameraFrameWire>,
}

fn camera_frame_to_wire(frame: &CameraFrame) -> CameraFrameWire {
    CameraFrameWire {
        width: frame.width,
        height: frame.height,
        stride: frame.stride,
        format: camera_pixel_format_to_u32(frame.format),
        bytes: frame.bytes.clone(),
    }
}

fn camera_frame_from_wire(frame: CameraFrameWire) -> Result<CameraFrame, CapabilityError> {
    Ok(CameraFrame {
        width: frame.width,
        height: frame.height,
        stride: frame.stride,
        format: camera_pixel_format_from_u32(frame.format)?,
        bytes: frame.bytes,
    })
}

fn face_bounds_to_wire(bounds: &FaceBounds) -> FaceBoundsWire {
    FaceBoundsWire {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    }
}

fn face_bounds_from_wire(bounds: FaceBoundsWire) -> FaceBounds {
    FaceBounds {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    }
}

fn biometric_modality_to_wire(modality: Option<BiometricModality>) -> u8 {
    match modality {
        Some(BiometricModality::Fingerprint) => 1,
        Some(BiometricModality::Face) => 2,
        Some(BiometricModality::Voice) => 3,
        Some(BiometricModality::Iris) => 4,
        Some(BiometricModality::Palm) => 5,
        Some(BiometricModality::Other(_)) => 255,
        None => 0,
    }
}

fn biometric_modality_from_wire(modality: u8) -> Option<BiometricModality> {
    match modality {
        0 => None,
        1 => Some(BiometricModality::Fingerprint),
        2 => Some(BiometricModality::Face),
        3 => Some(BiometricModality::Voice),
        4 => Some(BiometricModality::Iris),
        5 => Some(BiometricModality::Palm),
        _ => None,
    }
}

pub fn encode_camera_capture(capture: &CameraCapture) -> Vec<u8> {
    let wire = CameraCaptureWire {
        frame: camera_frame_to_wire(&capture.frame),
        quality: camera_capture_quality_to_u8(capture.quality),
        face_bounds: capture.face_bounds.as_ref().map(face_bounds_to_wire),
        state: BiometricStateWire {
            modality: biometric_modality_to_wire(capture.state.modality.clone()),
            verified: capture.state.verified,
            hardware_protected: capture.state.hardware_protected,
            user_present: capture.state.user_present,
        },
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .expect("camera capture must serialize through rkyv")
        .into_vec()
}

pub fn decode_camera_capture(bytes: &[u8]) -> Result<CameraCapture, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_wire::from_bytes::<CameraCaptureWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| CapabilityError::InvalidRequest("remote camera capture is not rkyv"))?;
    Ok(CameraCapture {
        frame: camera_frame_from_wire(wire.frame)?,
        quality: camera_capture_quality_from_u8(wire.quality)?,
        face_bounds: wire.face_bounds.map(face_bounds_from_wire),
        state: edgerun_biometrics::BiometricState {
            modality: biometric_modality_from_wire(wire.state.modality),
            verified: wire.state.verified,
            hardware_protected: wire.state.hardware_protected,
            user_present: wire.state.user_present,
        },
    })
}

pub fn encode_paired_camera_frame(frame: &PairedCameraFrame) -> Vec<u8> {
    let wire = PairedCameraFrameWire {
        rgb: frame.rgb.as_ref().map(camera_frame_to_wire),
        infrared: frame.infrared.as_ref().map(camera_frame_to_wire),
        depth: frame.depth.as_ref().map(camera_frame_to_wire),
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .expect("paired camera frame must serialize through rkyv")
        .into_vec()
}

pub fn decode_paired_camera_frame(bytes: &[u8]) -> Result<PairedCameraFrame, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_wire::from_bytes::<PairedCameraFrameWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| {
        CapabilityError::InvalidRequest("remote paired camera frame is not rkyv")
    })?;
    Ok(PairedCameraFrame {
        rgb: wire.rgb.map(camera_frame_from_wire).transpose()?,
        infrared: wire.infrared.map(camera_frame_from_wire).transpose()?,
        depth: wire.depth.map(camera_frame_from_wire).transpose()?,
    })
}

// --- CameraRemoteAdapter ---

#[derive(Debug)]
pub struct CameraRemoteAdapter<D> {
    pub device: D,
    pub purpose: CameraBiometricPurpose,
    pub timeout_ms: u32,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> CameraRemoteAdapter<D> {
    pub fn new(
        device: D,
        purpose: CameraBiometricPurpose,
        timeout_ms: u32,
        descriptor: CapabilityDescriptor,
    ) -> Self {
        Self {
            device,
            purpose,
            timeout_ms,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for CameraRemoteAdapter<D>
where
    D: CameraBiometricReader,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(stream_oriented_error(invocation, "camera"))
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self
            .device
            .capture(self.purpose, self.timeout_ms)
            .map_err(map_camera_error)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![CapabilityEventKind::Visual as i32],
            payload_object: None,
            inline_payload: encode_camera_capture(&capture),
        }))
    }
}

// --- PairedCameraRemoteAdapter ---

#[derive(Debug)]
pub struct PairedCameraRemoteAdapter<D> {
    pub device: D,
    pub purpose: CameraBiometricPurpose,
    pub timeout_ms: u32,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> PairedCameraRemoteAdapter<D> {
    pub fn new(
        device: D,
        purpose: CameraBiometricPurpose,
        timeout_ms: u32,
        descriptor: CapabilityDescriptor,
    ) -> Self {
        Self {
            device,
            purpose,
            timeout_ms,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for PairedCameraRemoteAdapter<D>
where
    D: PairedCameraBiometricReader,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(stream_oriented_error(invocation, "paired camera"))
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self
            .device
            .capture_paired(self.purpose, self.timeout_ms)
            .map_err(map_camera_error)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![CapabilityEventKind::Visual as i32],
            payload_object: None,
            inline_payload: encode_paired_camera_frame(&capture),
        }))
    }
}
