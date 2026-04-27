//! Camera biometric reader remote adapters (single-stream and paired RGB+IR+Depth).

use crate::prelude::v1::*;
use edgerun_biometrics::BiometricModality;
use edgerun_camera_biometrics::{
    CameraBiometricError, CameraBiometricPurpose, CameraBiometricReader, CameraCapture,
    CameraCaptureQuality, CameraFrame, CameraPixelFormat, CameraStreamRole, FaceBounds,
    PairedCameraBiometricReader, PairedCameraFrame,
};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_proto::edgerun::v0::capability::CapabilityInvocation;
use edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionEvent;

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
            ))
        }
    })
}

fn encode_camera_frame(frame: &CameraFrame) -> Vec<u8> {
    let mut out = Vec::with_capacity(20 + 4 + frame.bytes.len());
    out.extend_from_slice(&frame.width.to_le_bytes());
    out.extend_from_slice(&frame.height.to_le_bytes());
    out.extend_from_slice(&frame.stride.to_le_bytes());
    out.extend_from_slice(&camera_pixel_format_to_u32(frame.format).to_le_bytes());
    out.extend_from_slice(&(frame.bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame.bytes);
    out
}

fn decode_camera_frame(bytes: &[u8]) -> Result<(CameraFrame, usize), CapabilityError> {
    if bytes.len() < 20 {
        return Err(CapabilityError::InvalidRequest(
            "remote camera frame payload too short",
        ));
    }
    let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let stride = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let format =
        camera_pixel_format_from_u32(u32::from_le_bytes(bytes[12..16].try_into().unwrap()))?;
    let len = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    if bytes.len() < 20 + len {
        return Err(CapabilityError::InvalidRequest(
            "remote camera frame byte count exceeds payload length",
        ));
    }
    Ok((
        CameraFrame {
            width,
            height,
            stride,
            format,
            bytes: bytes[20..20 + len].to_vec(),
        },
        20 + len,
    ))
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
            ))
        }
    })
}

fn camera_stream_role_to_u8(role: CameraStreamRole) -> u8 {
    match role {
        CameraStreamRole::Unknown => 0,
        CameraStreamRole::Rgb => 1,
        CameraStreamRole::Infrared => 2,
        CameraStreamRole::Depth => 3,
        CameraStreamRole::Monochrome => 4,
    }
}

// --- Public encode/decode ---

pub fn encode_camera_capture(capture: &CameraCapture) -> Vec<u8> {
    let frame = encode_camera_frame(&capture.frame);
    let mut out = Vec::with_capacity(frame.len() + 8 + 20);
    out.extend_from_slice(&frame);
    out.push(camera_capture_quality_to_u8(capture.quality));
    out.push(capture.face_bounds.is_some() as u8);
    if let Some(bounds) = &capture.face_bounds {
        out.extend_from_slice(&bounds.x.to_le_bytes());
        out.extend_from_slice(&bounds.y.to_le_bytes());
        out.extend_from_slice(&bounds.width.to_le_bytes());
        out.extend_from_slice(&bounds.height.to_le_bytes());
    }
    out.push(capture.state.verified as u8);
    out.push(capture.state.hardware_protected as u8);
    out.push(capture.state.user_present as u8);
    out.push(match capture.state.modality {
        Some(BiometricModality::Fingerprint) => 1,
        Some(BiometricModality::Face) => 2,
        Some(BiometricModality::Voice) => 3,
        Some(BiometricModality::Iris) => 4,
        Some(BiometricModality::Palm) => 5,
        Some(BiometricModality::Other(_)) => 255,
        None => 0,
    });
    out
}

pub fn decode_camera_capture(bytes: &[u8]) -> Result<CameraCapture, CapabilityError> {
    let (frame, offset) = decode_camera_frame(bytes)?;
    if bytes.len() < offset + 5 {
        return Err(CapabilityError::InvalidRequest(
            "remote camera capture payload too short",
        ));
    }
    let quality = camera_capture_quality_from_u8(bytes[offset])?;
    let has_bounds = bytes[offset + 1] != 0;
    let mut cursor = offset + 2;
    let face_bounds = if has_bounds {
        if bytes.len() < cursor + 16 + 4 {
            return Err(CapabilityError::InvalidRequest(
                "remote camera face bounds payload too short",
            ));
        }
        let x = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        let y = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        let width = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        let height = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        Some(FaceBounds {
            x,
            y,
            width,
            height,
        })
    } else {
        None
    };
    if bytes.len() != cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote camera capture payload length is invalid",
        ));
    }
    let modality = match bytes[cursor + 3] {
        0 => None,
        1 => Some(BiometricModality::Fingerprint),
        2 => Some(BiometricModality::Face),
        3 => Some(BiometricModality::Voice),
        4 => Some(BiometricModality::Iris),
        5 => Some(BiometricModality::Palm),
        _ => None,
    };
    Ok(CameraCapture {
        frame,
        quality,
        face_bounds,
        state: edgerun_biometrics::BiometricState {
            modality,
            verified: bytes[cursor] != 0,
            hardware_protected: bytes[cursor + 1] != 0,
            user_present: bytes[cursor + 2] != 0,
        },
    })
}

pub fn encode_paired_camera_frame(frame: &PairedCameraFrame) -> Vec<u8> {
    let mut out = Vec::new();
    for (role, maybe_frame) in [
        (CameraStreamRole::Rgb, frame.rgb.as_ref()),
        (CameraStreamRole::Infrared, frame.infrared.as_ref()),
        (CameraStreamRole::Depth, frame.depth.as_ref()),
    ] {
        out.push(maybe_frame.is_some() as u8);
        if let Some(inner) = maybe_frame {
            out.push(camera_stream_role_to_u8(role));
            out.extend_from_slice(&encode_camera_frame(inner));
        }
    }
    out
}

pub fn decode_paired_camera_frame(bytes: &[u8]) -> Result<PairedCameraFrame, CapabilityError> {
    let mut cursor = 0usize;
    let mut rgb = None;
    let mut infrared = None;
    let mut depth = None;
    for slot in 0..3 {
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote paired camera payload too short",
            ));
        }
        let present = bytes[cursor] != 0;
        cursor += 1;
        if present {
            if bytes.len() < cursor + 1 {
                return Err(CapabilityError::InvalidRequest(
                    "remote paired camera role missing",
                ));
            }
            let role = bytes[cursor];
            cursor += 1;
            let (frame, used) = decode_camera_frame(&bytes[cursor..])?;
            cursor += used;
            match role {
                1 => rgb = Some(frame),
                2 => infrared = Some(frame),
                3 => depth = Some(frame),
                4 if slot == 1 => infrared = Some(frame),
                _ => {}
            }
        }
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote paired camera payload trailing bytes are invalid",
        ));
    }
    Ok(PairedCameraFrame {
        rgb,
        infrared,
        depth,
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
