//! Additional remote adapters for display, NPU, and fingerprint devices.

use edgerun_capabilities::{
    CapabilityAccessClass, CapabilityDescriptor, CapabilityError, CapabilityEventKind,
};
use edgerun_display::{
    DisplayContentKind, DisplayDevice, DisplayInfo, DisplayUpdateRequest,
};
use edgerun_fingerprint::{
    FingerprintCapture, FingerprintCapturePurpose, FingerprintCaptureQuality, FingerprintError,
    FingerprintReader, FingerprintReaderInfo,
};
use edgerun_npu::{NpuDevice, NpuExecutionMode, NpuInfo, NpuWorkloadRequest, NpuWorkloadResult};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionEvent, CapabilitySessionOpen,
};

use crate::{
    accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult,
};

// ===========================================================================
// Display Remote Adapter
// ===========================================================================

#[derive(Debug)]
pub struct DisplayRemoteAdapter<D> {
    pub device: D,
}

impl<D> DisplayRemoteAdapter<D> {
    pub fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D: DisplayDevice> RemoteCapabilityProvider for DisplayRemoteAdapter<D> {
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let op = edgerun_capabilities::CapabilityOperation::try_from(invocation.operation)
            .map_err(|_| CapabilityError::InvalidRequest("invalid operation"))?;
        match op {
            edgerun_capabilities::CapabilityOperation::Query => {
                let info = self.device.display_info().map_err(|e| {
                    CapabilityError::Provider(format!("display query failed: {e}"))
                })?;
                let payload = serde_json_display_info(&info);
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Display as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: payload,
                })
            }
            edgerun_capabilities::CapabilityOperation::Render => {
                let params = inline_parameters.ok_or(CapabilityError::InvalidRequest(
                    "display render requires inline parameters",
                ))?;
                let request = decode_display_update_request(&params)?;
                self.device.present(&request).map_err(|e| {
                    CapabilityError::Provider(format!("display present failed: {e}"))
                })?;
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Display as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: b"display-presented".to_vec(),
                })
            }
            _ => Err(CapabilityError::Unsupported(
                "display adapter does not support this operation",
            )),
        }
    }
}

fn decode_display_update_request(bytes: &[u8]) -> Result<DisplayUpdateRequest, CapabilityError> {
    if bytes.len() < 13 {
        return Err(CapabilityError::InvalidRequest("display update request too short"));
    }
    let content_kind = match bytes[0] {
        0 => DisplayContentKind::Text,
        1 => DisplayContentKind::Image,
        2 => DisplayContentKind::Video,
        3 => DisplayContentKind::Prompt,
        v => DisplayContentKind::Other(v as u32),
    };
    let width = if bytes[1] != 0 {
        Some(u32::from_le_bytes(bytes[2..6].try_into().unwrap()))
    } else {
        None
    };
    let height = if bytes[6] != 0 {
        Some(u32::from_le_bytes(bytes[7..11].try_into().unwrap()))
    } else {
        None
    };
    let refresh = if bytes[11] != 0 {
        Some(u32::from_le_bytes(bytes[12..16].try_into().unwrap()))
    } else {
        None
    };
    Ok(DisplayUpdateRequest { content_kind, width, height, refresh_millihz: refresh })
}

fn serde_json_display_info(info: &DisplayInfo) -> Vec<u8> {
    format!(
        "{{\"display_name\":\"{}\",\"width\":{},\"height\":{},\"refresh_millihz\":{},\"hdr\":{},\"touch\":{}}}",
        info.display_name,
        info.current_mode.width,
        info.current_mode.height,
        info.current_mode.refresh_millihz,
        info.hdr_capable,
        info.touch_capable,
    )
    .into_bytes()
}

// ===========================================================================
// NPU Remote Adapter
// ===========================================================================

#[derive(Debug)]
pub struct NpuRemoteAdapter<D> {
    pub device: D,
}

impl<D> NpuRemoteAdapter<D> {
    pub fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D: NpuDevice> RemoteCapabilityProvider for NpuRemoteAdapter<D> {
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let op = edgerun_capabilities::CapabilityOperation::try_from(invocation.operation)
            .map_err(|_| CapabilityError::InvalidRequest("invalid operation"))?;
        match op {
            edgerun_capabilities::CapabilityOperation::Query => {
                let info = self.device.npu_info().map_err(|e| {
                    CapabilityError::Provider(format!("npu info query failed: {e}"))
                })?;
                let payload = serde_json_npu_info(&info);
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Inference as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: payload,
                })
            }
            edgerun_capabilities::CapabilityOperation::Invoke => {
                let params = inline_parameters.ok_or(CapabilityError::InvalidRequest(
                    "npu workload requires inline parameters",
                ))?;
                let request = decode_npu_workload_request(&params)?;
                let result = self.device.execute_workload(&request).map_err(|e| {
                    CapabilityError::Provider(format!("npu workload execution failed: {e}"))
                })?;
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Inference as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: encode_npu_workload_result(&result),
                })
            }
            _ => Err(CapabilityError::Unsupported(
                "npu adapter does not support this operation",
            )),
        }
    }
}

fn decode_npu_workload_request(bytes: &[u8]) -> Result<NpuWorkloadRequest, CapabilityError> {
    if bytes.len() < 5 {
        return Err(CapabilityError::InvalidRequest("npu workload request too short"));
    }
    let mode = match bytes[0] {
        0 => NpuExecutionMode::Inference,
        1 => NpuExecutionMode::Compilation,
        2 => NpuExecutionMode::Preprocessing,
        v => NpuExecutionMode::Other(v as u32),
    };
    let input_len = u32::from_le_bytes(bytes[1..5].try_into().unwrap()) as usize;
    let input_bytes = if bytes.len() >= 5 + input_len {
        bytes[5..5 + input_len].to_vec()
    } else {
        Vec::new()
    };
    let target_latency_ms = if bytes.len() >= 5 + input_len + 4 {
        Some(u32::from_le_bytes(bytes[5 + input_len..5 + input_len + 4].try_into().unwrap()))
    } else {
        None
    };
    Ok(NpuWorkloadRequest { mode, input_bytes, target_latency_ms })
}

fn encode_npu_workload_result(result: &NpuWorkloadResult) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(result.bytes_processed as u32).to_le_bytes());
    out.push(if result.completed { 1 } else { 0 });
    out
}

fn serde_json_npu_info(info: &NpuInfo) -> Vec<u8> {
    format!(
        "{{\"display_name\":\"{}\",\"driver\":\"{}\",\"pci\":\"{}\",\"firmware\":\"{}\",\"supports_submission\":{}}}",
        info.display_name,
        info.driver_name.as_deref().unwrap_or("unknown"),
        info.pci_address.as_deref().unwrap_or("unknown"),
        info.firmware_version.as_deref().unwrap_or("unknown"),
        info.supports_submission,
    )
    .into_bytes()
}

// ===========================================================================
// Fingerprint Remote Adapter
// ===========================================================================

#[derive(Debug)]
pub struct FingerprintRemoteAdapter<D> {
    pub device: D,
    pub timeout_ms: u32,
    next_sequence_no: u64,
}

impl<D> FingerprintRemoteAdapter<D> {
    pub fn new(device: D, timeout_ms: u32) -> Self {
        Self { device, timeout_ms, next_sequence_no: 1 }
    }
}

fn map_fingerprint_error(e: FingerprintError) -> CapabilityError {
    match e {
        FingerprintError::InvalidRequest(msg) => CapabilityError::InvalidRequest(msg),
        FingerprintError::InvalidState(msg) => CapabilityError::InvalidRequest(msg),
        FingerprintError::UnsupportedOperation(msg) => CapabilityError::Unsupported(msg),
        FingerprintError::Provider(msg) => CapabilityError::Provider(msg),
    }
}

fn encode_fingerprint_capture(
    capture: &FingerprintCapture,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(capture.bytes.len() + 2);
    let quality_byte = match capture.quality {
        FingerprintCaptureQuality::Poor => 0u8,
        FingerprintCaptureQuality::Fair => 1,
        FingerprintCaptureQuality::Good => 2,
        FingerprintCaptureQuality::Excellent => 3,
    };
    out.push(quality_byte);
    out.push(if capture.state.verified { 1 } else { 0 });
    out.extend_from_slice(&capture.bytes);
    out
}

impl<D: FingerprintReader> RemoteCapabilityProvider for FingerprintRemoteAdapter<D> {
    fn descriptor(&self) -> CapabilityDescriptor {
        let info = self.device.reader_info().ok();
        let instance_id = info
            .as_ref()
            .map(|i| i.reader_name.clone())
            .unwrap_or_else(|| "fingerprint".into());
        edgerun_capabilities::capability_descriptor(
            "edgerun-fingerprint",
            &instance_id,
            edgerun_capabilities::CapabilityRole::SecureElement,
            &[edgerun_capabilities::CapabilityModality::Biometric],
            &[CapabilityEventKind::Biometric],
            &[
                edgerun_capabilities::CapabilityOperation::Query,
                edgerun_capabilities::CapabilityOperation::Capture,
                edgerun_capabilities::CapabilityOperation::Verify,
            ],
            vec![edgerun_capabilities::constraint(
                edgerun_capabilities::CapabilityConstraintKind::RequireLocalOnly,
            )],
        )
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let op = edgerun_capabilities::CapabilityOperation::try_from(invocation.operation)
            .map_err(|_| CapabilityError::InvalidRequest("invalid operation"))?;
        match op {
            edgerun_capabilities::CapabilityOperation::Query => {
                let info = self.device.reader_info().map_err(map_fingerprint_error)?;
                let payload = serde_json_fingerprint_info(&info);
                Ok(RemoteInvocationResult {
                    result: CapabilityResult {
                        result_version: 1,
                        invocation_id: invocation.invocation_id.clone(),
                        grant_id: invocation.grant_id.clone(),
                        success: true,
                        result_access_class: CapabilityAccessClass::Derived as i32,
                        produced_event_kinds: vec![CapabilityEventKind::Biometric as i32],
                        payload_object: None,
                        error_reason: String::new(),
                        produced_at: None,
                        signature: None,
                    },
                    inline_payload: payload,
                })
            }
            _ => Err(CapabilityError::Unsupported(
                "fingerprint adapter: use session events for capture",
            )),
        }
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self
            .device
            .capture(FingerprintCapturePurpose::Verification, self.timeout_ms)
            .map_err(map_fingerprint_error)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![CapabilityEventKind::Biometric as i32],
            payload_object: None,
            inline_payload: encode_fingerprint_capture(&capture),
        }))
    }
}

fn serde_json_fingerprint_info(info: &FingerprintReaderInfo) -> Vec<u8> {
    format!(
        "{{\"reader_name\":\"{}\",\"max_templates\":{},\"hardware_protected\":{},\"match_on_sensor\":{}}}",
        info.reader_name,
        info.max_templates.unwrap_or(0),
        info.hardware_protected_match,
        info.supports_match_on_sensor,
    )
    .into_bytes()
}
