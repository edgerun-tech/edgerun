// Re-export types that downstream NPU backends need to implement traits.
pub use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityProvider,
};

use edgerun_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityEventKind,
    CapabilityModality, CapabilityOperation, CapabilityRole,
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
    fn npu_execution_mode_variants_are_copy() {
        let mode = NpuExecutionMode::Inference;
        let _copied = mode;
        assert_eq!(mode, NpuExecutionMode::Inference);
    }

    #[test]
    fn npu_execution_mode_all_variants() {
        assert_eq!(NpuExecutionMode::Inference, NpuExecutionMode::Inference);
        assert_eq!(NpuExecutionMode::Compilation, NpuExecutionMode::Compilation);
        assert_eq!(NpuExecutionMode::Preprocessing, NpuExecutionMode::Preprocessing);
        assert_eq!(
            NpuExecutionMode::Other(42),
            NpuExecutionMode::Other(42)
        );
        assert_ne!(NpuExecutionMode::Inference, NpuExecutionMode::Other(0));
        assert_ne!(
            NpuExecutionMode::Other(1),
            NpuExecutionMode::Other(2)
        );
    }

    #[test]
    fn npu_execution_mode_debug() {
        assert_eq!(format!("{:?}", NpuExecutionMode::Inference), "Inference");
        assert_eq!(
            format!("{:?}", NpuExecutionMode::Other(99)),
            "Other(99)"
        );
    }

    #[test]
    fn npu_info_construction() {
        let info = NpuInfo {
            provider: "test-npu".into(),
            instance_id: "npu0".into(),
            display_name: "Test NPU".into(),
            driver_name: Some("test-driver".into()),
            pci_address: Some("0000:01:00.0".into()),
            vendor_id: Some(0x1234),
            device_id: Some(0x5678),
            character_device: Some("/dev/accel0".into()),
            firmware_version: Some("v1.0".into()),
            supports_submission: true,
        };
        assert_eq!(info.provider, "test-npu");
        assert_eq!(info.instance_id, "npu0");
        assert_eq!(info.display_name, "Test NPU");
        assert!(info.supports_submission);
    }

    #[test]
    fn npu_info_with_none_fields() {
        let info = NpuInfo {
            provider: "test".into(),
            instance_id: "npu0".into(),
            display_name: "NPU".into(),
            driver_name: None,
            pci_address: None,
            vendor_id: None,
            device_id: None,
            character_device: None,
            firmware_version: None,
            supports_submission: false,
        };
        assert_eq!(info.driver_name, None);
        assert!(!info.supports_submission);
    }

    #[test]
    fn npu_info_clone() {
        let info = NpuInfo {
            provider: "p".into(),
            instance_id: "i".into(),
            display_name: "d".into(),
            driver_name: None,
            pci_address: None,
            vendor_id: None,
            device_id: None,
            character_device: None,
            firmware_version: None,
            supports_submission: false,
        };
        assert_eq!(info.clone(), info);
    }

    #[test]
    fn npu_workload_request_construction() {
        let req = NpuWorkloadRequest {
            mode: NpuExecutionMode::Inference,
            input_bytes: vec![1, 2, 3],
            target_latency_ms: Some(16),
        };
        assert_eq!(req.input_bytes.len(), 3);
        assert_eq!(req.target_latency_ms, Some(16));
    }

    #[test]
    fn npu_workload_request_clone() {
        let req = NpuWorkloadRequest {
            mode: NpuExecutionMode::Compilation,
            input_bytes: vec![0],
            target_latency_ms: None,
        };
        assert_eq!(req.clone(), req);
    }

    #[test]
    fn npu_workload_result_construction() {
        let result = NpuWorkloadResult {
            bytes_processed: 1024,
            completed: true,
        };
        assert_eq!(result.bytes_processed, 1024);
        assert!(result.completed);
    }

    #[test]
    fn npu_workload_result_clone() {
        let result = NpuWorkloadResult {
            bytes_processed: 0,
            completed: false,
        };
        assert_eq!(result.clone(), result);
    }

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
    fn descriptor_contains_query_and_invoke_operations() {
        let descriptor = default_npu_descriptor("linux-npu", "accel0");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Invoke as i32)));
    }

    #[test]
    fn descriptor_contains_hardware_protection_constraint() {
        let descriptor = default_npu_descriptor("linux-npu", "accel0");
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireHardwareProtected as i32));
    }

    #[test]
    fn descriptor_contains_require_local_constraint() {
        let descriptor = default_npu_descriptor("linux-npu", "accel0");
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireLocalOnly as i32));
    }

    #[test]
    fn descriptor_stores_provider_in_name_and_instance_fields() {
        let descriptor = default_npu_descriptor("custom-npu", "npu-custom-1");
        assert_eq!(descriptor.provider_name, "custom-npu");
        assert_eq!(descriptor.provider_instance_id, "npu-custom-1");
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

    #[test]
    fn workload_request_valid_with_bytes() {
        let result = validate_npu_workload_request(&NpuWorkloadRequest {
            mode: NpuExecutionMode::Inference,
            input_bytes: vec![1, 2, 3],
            target_latency_ms: None,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn workload_request_valid_all_modes() {
        for mode in [
            NpuExecutionMode::Inference,
            NpuExecutionMode::Compilation,
            NpuExecutionMode::Preprocessing,
            NpuExecutionMode::Other(99),
        ] {
            let result = validate_npu_workload_request(&NpuWorkloadRequest {
                mode,
                input_bytes: vec![0],
                target_latency_ms: Some(100),
            });
            assert!(result.is_ok(), "mode {:?} should be valid", mode);
        }
    }
}
