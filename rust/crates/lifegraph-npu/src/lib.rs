use lifegraph_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpuExecutionMode {
    Inference,
    Compilation,
    Preprocessing,
    Other(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpuInfo {
    pub provider: String,
    pub instance_id: String,
    pub display_name: String,
    pub driver_name: Option<String>,
    pub pci_address: Option<String>,
    pub vendor_id: Option<u32>,
    pub device_id: Option<u32>,
    pub character_device: Option<String>,
    pub firmware_version: Option<String>,
    pub supports_submission: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpuWorkloadRequest {
    pub mode: NpuExecutionMode,
    pub input_bytes: Vec<u8>,
    pub target_latency_ms: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpuWorkloadResult {
    pub bytes_processed: usize,
    pub completed: bool,
}

pub trait NpuDevice: CapabilityProvider {
    fn npu_info(&self) -> Result<NpuInfo, CapabilityError>;
    fn execute_workload(
        &self,
        request: &NpuWorkloadRequest,
    ) -> Result<NpuWorkloadResult, CapabilityError>;
}

pub fn default_npu_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Execution,
        &[CapabilityModality::Computational],
        &[CapabilityEventKind::Inference],
        &[CapabilityOperation::Query, CapabilityOperation::Invoke],
        vec![
            constraint(CapabilityConstraintKind::RequireLocalOnly),
            constraint(CapabilityConstraintKind::RequireHardwareProtected),
        ],
    )
}

pub fn validate_npu_workload_request(request: &NpuWorkloadRequest) -> Result<(), CapabilityError> {
    if request.input_bytes.is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "npu workload input bytes must be non-empty",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_execution_compute_inference() {
        let descriptor = default_npu_descriptor("linux-npu", "accel0");
        assert_eq!(descriptor.role, CapabilityRole::Execution as i32);
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Computational as i32]
        );
        assert_eq!(
            descriptor.event_kinds,
            vec![CapabilityEventKind::Inference as i32]
        );
    }

    #[test]
    fn workload_request_requires_bytes() {
        let err = validate_npu_workload_request(&NpuWorkloadRequest {
            mode: NpuExecutionMode::Inference,
            input_bytes: Vec::new(),
            target_latency_ms: None,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("npu workload input bytes must be non-empty")
        );
    }
}
