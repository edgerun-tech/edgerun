use lifegraph_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    Qualcomm,
    Apple,
    Arm,
    Matrox,
    Aspeed,
    Virtio,
    Microsoft,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuDisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuConnectorInfo {
    pub name: String,
    pub connector_id: Option<u32>,
    pub connected: bool,
    pub enabled: bool,
    pub current_mode: Option<GpuDisplayMode>,
    pub modes: Vec<GpuDisplayMode>,
    pub cec_adapter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuInfo {
    pub provider: String,
    pub instance_id: String,
    pub vendor: GpuVendor,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,
    pub subsystem_vendor_id: Option<u16>,
    pub subsystem_device_id: Option<u16>,
    pub pci_address: Option<String>,
    pub drm_cards: Vec<String>,
    pub drm_render_nodes: Vec<String>,
    pub drm_card_device_nodes: Vec<String>,
    pub drm_render_device_nodes: Vec<String>,
    pub connectors: Vec<GpuConnectorInfo>,
    pub driver: Option<String>,
    pub driver_module: Option<String>,
    pub modalias: Option<String>,
    pub class_code: Option<u32>,
    pub revision: Option<u8>,
    pub numa_node: Option<i32>,
    pub current_link_speed: Option<String>,
    pub current_link_width: Option<u32>,
    pub max_link_speed: Option<String>,
    pub max_link_width: Option<u32>,
    pub boot_vga: bool,
    pub supports_display: bool,
    pub supports_render: bool,
    pub supports_compute: bool,
    pub is_virtual: bool,
}

pub trait GpuInventory: CapabilityProvider {
    fn list_gpus(&self) -> Result<Vec<GpuInfo>, CapabilityError>;
}

pub fn default_gpu_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Execution,
        &[
            CapabilityModality::Display,
            CapabilityModality::Visual,
            CapabilityModality::Computational,
        ],
        &[
            CapabilityEventKind::Display,
            CapabilityEventKind::Inference,
            CapabilityEventKind::State,
        ],
        &[CapabilityOperation::Query, CapabilityOperation::Observe],
        vec![constraint(CapabilityConstraintKind::RequireLocalOnly)],
    )
}

pub fn infer_gpu_vendor(vendor_id: Option<u16>) -> GpuVendor {
    match vendor_id {
        Some(0x1002) | Some(0x1022) => GpuVendor::Amd,
        Some(0x8086) => GpuVendor::Intel,
        Some(0x10de) => GpuVendor::Nvidia,
        Some(0x17cb) | Some(0x5143) => GpuVendor::Qualcomm,
        Some(0x106b) => GpuVendor::Apple,
        Some(0x13b5) => GpuVendor::Arm,
        Some(0x102b) => GpuVendor::Matrox,
        Some(0x1a03) => GpuVendor::Aspeed,
        Some(0x1af4) => GpuVendor::Virtio,
        Some(0x1414) => GpuVendor::Microsoft,
        _ => GpuVendor::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestInventory;

    impl CapabilityProvider for TestInventory {
        fn descriptor(&self) -> CapabilityDescriptor {
            default_gpu_descriptor("test-gpu", "root")
        }
    }

    impl GpuInventory for TestInventory {
        fn list_gpus(&self) -> Result<Vec<GpuInfo>, CapabilityError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn descriptor_is_local_execution_capability() {
        let descriptor = default_gpu_descriptor("linux-gpu", "0000:01:00.0");
        assert_eq!(descriptor.role, CapabilityRole::Execution as i32);
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Display as i32)));
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Computational as i32)));
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireLocalOnly as i32));
    }

    #[test]
    fn infer_vendor_from_known_ids() {
        assert_eq!(infer_gpu_vendor(Some(0x10de)), GpuVendor::Nvidia);
        assert_eq!(infer_gpu_vendor(Some(0x8086)), GpuVendor::Intel);
        assert_eq!(infer_gpu_vendor(Some(0x1af4)), GpuVendor::Virtio);
    }

    #[test]
    fn inventory_trait_is_object_safe_enough_for_basic_use() {
        let inventory = TestInventory;
        assert!(inventory.list_gpus().unwrap().is_empty());
    }
}
