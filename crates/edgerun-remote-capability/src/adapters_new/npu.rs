//! NPU device remote adapter.

use edgerun_capabilities::{
    CapabilityAccessClass, CapabilityDescriptor, CapabilityError, CapabilityEventKind,
    CapabilityOperation,
};
use edgerun_npu::{NpuDevice, NpuExecutionMode, NpuInfo, NpuWorkloadRequest, NpuWorkloadResult};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionOpen,
};

use crate::protocol::{accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult};

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

    fn open_session(&mut self, open: &CapabilitySessionOpen) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(&mut self, _session_id: &[u8], invocation: &CapabilityInvocation, inline_parameters: Option<&[u8]>) -> Result<RemoteInvocationResult, CapabilityError> {
        let op = CapabilityOperation::try_from(invocation.operation)
            .map_err(|_| CapabilityError::InvalidRequest("invalid operation"))?;
        match op {
            CapabilityOperation::Query => {
                let info = self.device.npu_info().map_err(|e| {
                    CapabilityError::Provider(format!("npu info query failed: {e}"))
                })?;
                let payload = npu_info_to_json(&info);
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
            CapabilityOperation::Invoke => {
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
    } else { Vec::new() };
    let target_latency_ms = if bytes.len() >= 5 + input_len + 4 {
        Some(u32::from_le_bytes(bytes[5 + input_len..5 + input_len + 4].try_into().unwrap()))
    } else { None };
    Ok(NpuWorkloadRequest { mode, input_bytes, target_latency_ms })
}

fn encode_npu_workload_result(result: &NpuWorkloadResult) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(result.bytes_processed as u32).to_le_bytes());
    out.push(if result.completed { 1 } else { 0 });
    out
}

fn npu_info_to_json(info: &NpuInfo) -> Vec<u8> {
    format!(
        "{{\"display_name\":\"{}\",\"driver\":\"{}\",\"pci\":\"{}\",\"firmware\":\"{}\",\"supports_submission\":{}}}",
        info.display_name,
        info.driver_name.as_deref().unwrap_or("unknown"),
        info.pci_address.as_deref().unwrap_or("unknown"),
        info.firmware_version.as_deref().unwrap_or("unknown"),
        info.supports_submission,
    ).into_bytes()
}
