//! Fingerprint reader remote adapter.

use edgerun_capabilities::{
    capability_descriptor, constraint, CapabilityAccessClass,
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityRole, CapabilityConstraintKind,
};
use edgerun_fingerprint::{
    FingerprintCapture, FingerprintCapturePurpose, FingerprintCaptureQuality, FingerprintError,
    FingerprintReader, FingerprintReaderInfo,
};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionEvent, CapabilitySessionOpen,
};

use crate::protocol::{accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult};

fn map_fingerprint_error(e: FingerprintError) -> CapabilityError {
    match e {
        FingerprintError::InvalidRequest(msg) => CapabilityError::InvalidRequest(msg),
        FingerprintError::InvalidState(msg) => CapabilityError::InvalidRequest(msg),
        FingerprintError::UnsupportedOperation(msg) => CapabilityError::Unsupported(msg),
        FingerprintError::Provider(msg) => CapabilityError::Provider(msg),
    }
}

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

impl<D: FingerprintReader> RemoteCapabilityProvider for FingerprintRemoteAdapter<D> {
    fn descriptor(&self) -> CapabilityDescriptor {
        let info = self.device.reader_info().ok();
        let instance_id = info
            .as_ref()
            .map(|i| i.reader_name.clone())
            .unwrap_or_else(|| "fingerprint".into());
        capability_descriptor(
            "edgerun-fingerprint",
            &instance_id,
            CapabilityRole::SecureElement,
            &[CapabilityModality::Biometric],
            &[CapabilityEventKind::Biometric],
            &[
                CapabilityOperation::Query,
                CapabilityOperation::Capture,
                CapabilityOperation::Verify,
            ],
            vec![constraint(CapabilityConstraintKind::RequireLocalOnly)],
        )
    }

    fn open_session(&mut self, open: &CapabilitySessionOpen) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(&mut self, _session_id: &[u8], invocation: &CapabilityInvocation, _inline_parameters: Option<&[u8]>) -> Result<RemoteInvocationResult, CapabilityError> {
        let op = CapabilityOperation::try_from(invocation.operation)
            .map_err(|_| CapabilityError::InvalidRequest("invalid operation"))?;
        match op {
            CapabilityOperation::Query => {
                let info = self.device.reader_info().map_err(map_fingerprint_error)?;
                let payload = fingerprint_info_to_json(&info);
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

    fn next_event(&mut self, session_id: &[u8]) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
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

fn encode_fingerprint_capture(capture: &FingerprintCapture) -> Vec<u8> {
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

fn fingerprint_info_to_json(info: &FingerprintReaderInfo) -> Vec<u8> {
    format!(
        "{{\"reader_name\":\"{}\",\"max_templates\":{},\"hardware_protected\":{},\"match_on_sensor\":{}}}",
        info.reader_name,
        info.max_templates.unwrap_or(0),
        info.hardware_protected_match,
        info.supports_match_on_sensor,
    ).into_bytes()
}
