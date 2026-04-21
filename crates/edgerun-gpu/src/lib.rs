use edgerun_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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
    #[default]
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
    fn gpu_vendor_variants_are_copy() {
        let vendor = GpuVendor::Nvidia;
        let _copied = vendor;
        assert_eq!(vendor, GpuVendor::Nvidia);
    }

    #[test]
    fn gpu_vendor_all_variants_distinct() {
        let vendors = [
            GpuVendor::Amd,
            GpuVendor::Intel,
            GpuVendor::Nvidia,
            GpuVendor::Qualcomm,
            GpuVendor::Apple,
            GpuVendor::Arm,
            GpuVendor::Matrox,
            GpuVendor::Aspeed,
            GpuVendor::Virtio,
            GpuVendor::Microsoft,
            GpuVendor::Unknown,
        ];
        // All should be distinct
        for (i, v1) in vendors.iter().enumerate() {
            for (j, v2) in vendors.iter().enumerate() {
                if i != j {
                    assert_ne!(*v1, *v2, "{:?} should not equal {:?}", v1, v2);
                }
            }
        }
    }

    #[test]
    fn gpu_vendor_debug() {
        assert_eq!(format!("{:?}", GpuVendor::Amd), "Amd");
        assert_eq!(format!("{:?}", GpuVendor::Unknown), "Unknown");
    }

    #[test]
    fn gpu_display_mode_copy() {
        let mode = GpuDisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let _copied = mode;
        assert_eq!(mode.width, 1920);
    }

    #[test]
    fn gpu_display_mode_equality() {
        let m1 = GpuDisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let m2 = GpuDisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let m3 = GpuDisplayMode {
            width: 3840,
            height: 2160,
            refresh_millihz: 60_000,
        };
        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }

    #[test]
    fn gpu_display_mode_debug() {
        let mode = GpuDisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let debug = format!("{:?}", mode);
        assert!(debug.contains("1920"));
        assert!(debug.contains("1080"));
    }

    #[test]
    fn gpu_connector_info_construction() {
        let info = GpuConnectorInfo {
            name: "HDMI-A-1".into(),
            connector_id: Some(128),
            connected: true,
            enabled: true,
            current_mode: Some(GpuDisplayMode {
                width: 3840,
                height: 2160,
                refresh_millihz: 60_000,
            }),
            modes: vec![GpuDisplayMode {
                width: 1920,
                height: 1080,
                refresh_millihz: 60_000,
            }],
            cec_adapter: Some("cec0".into()),
        };
        assert_eq!(info.name, "HDMI-A-1");
        assert!(info.connected);
        assert_eq!(info.modes.len(), 1);
    }

    #[test]
    fn gpu_connector_info_clone() {
        let info = GpuConnectorInfo {
            name: "DP-1".into(),
            connector_id: None,
            connected: false,
            enabled: false,
            current_mode: None,
            modes: Vec::new(),
            cec_adapter: None,
        };
        assert_eq!(info.clone(), info);
    }

    #[test]
    fn gpu_info_construction() {
        let info = GpuInfo {
            provider: "test".into(),
            instance_id: "gpu0".into(),
            vendor: GpuVendor::Nvidia,
            vendor_id: Some(0x10de),
            device_id: Some(0x1cb3),
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            pci_address: Some("0000:01:00.0".into()),
            drm_cards: vec!["card0".into()],
            drm_render_nodes: vec!["renderD128".into()],
            drm_card_device_nodes: vec![],
            drm_render_device_nodes: vec![],
            connectors: vec![],
            driver: Some("nvidia".into()),
            driver_module: None,
            modalias: None,
            class_code: Some(0x030000),
            revision: None,
            numa_node: None,
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            boot_vga: true,
            supports_display: true,
            supports_render: true,
            supports_compute: true,
            is_virtual: false,
        };
        assert_eq!(info.provider, "test");
        assert_eq!(info.vendor, GpuVendor::Nvidia);
        assert!(info.boot_vga);
        assert!(!info.is_virtual);
    }

    #[test]
    fn gpu_info_clone() {
        let info = GpuInfo {
            provider: "p".into(),
            instance_id: "i".into(),
            vendor: GpuVendor::Unknown,
            vendor_id: None,
            device_id: None,
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            pci_address: None,
            drm_cards: vec![],
            drm_render_nodes: vec![],
            drm_card_device_nodes: vec![],
            drm_render_device_nodes: vec![],
            connectors: vec![],
            driver: None,
            driver_module: None,
            modalias: None,
            class_code: None,
            revision: None,
            numa_node: None,
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            boot_vga: false,
            supports_display: false,
            supports_render: false,
            supports_compute: false,
            is_virtual: false,
        };
        assert_eq!(info.clone(), info);
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
    fn descriptor_contains_visual_modality() {
        let descriptor = default_gpu_descriptor("linux-gpu", "gpu0");
        assert!(descriptor
            .modalities
            .contains(&(CapabilityModality::Visual as i32)));
    }

    #[test]
    fn descriptor_contains_display_and_inference_events() {
        let descriptor = default_gpu_descriptor("linux-gpu", "gpu0");
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Display as i32)));
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Inference as i32)));
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }

    #[test]
    fn descriptor_contains_query_and_observe_operations() {
        let descriptor = default_gpu_descriptor("linux-gpu", "gpu0");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
    }

    #[test]
    fn descriptor_stores_provider_in_name_and_instance_fields() {
        let descriptor = default_gpu_descriptor("custom-gpu", "gpu-custom-1");
        assert_eq!(descriptor.provider_name, "custom-gpu");
        assert_eq!(descriptor.provider_instance_id, "gpu-custom-1");
    }

    #[test]
    fn infer_vendor_from_known_ids() {
        assert_eq!(infer_gpu_vendor(Some(0x10de)), GpuVendor::Nvidia);
        assert_eq!(infer_gpu_vendor(Some(0x8086)), GpuVendor::Intel);
        assert_eq!(infer_gpu_vendor(Some(0x1af4)), GpuVendor::Virtio);
    }

    #[test]
    fn infer_vendor_amd_ids() {
        assert_eq!(infer_gpu_vendor(Some(0x1002)), GpuVendor::Amd);
        assert_eq!(infer_gpu_vendor(Some(0x1022)), GpuVendor::Amd);
    }

    #[test]
    fn infer_vendor_qualcomm_ids() {
        assert_eq!(infer_gpu_vendor(Some(0x17cb)), GpuVendor::Qualcomm);
        assert_eq!(infer_gpu_vendor(Some(0x5143)), GpuVendor::Qualcomm);
    }

    #[test]
    fn infer_vendor_all_known_ids() {
        assert_eq!(infer_gpu_vendor(Some(0x106b)), GpuVendor::Apple);
        assert_eq!(infer_gpu_vendor(Some(0x13b5)), GpuVendor::Arm);
        assert_eq!(infer_gpu_vendor(Some(0x102b)), GpuVendor::Matrox);
        assert_eq!(infer_gpu_vendor(Some(0x1a03)), GpuVendor::Aspeed);
        assert_eq!(infer_gpu_vendor(Some(0x1414)), GpuVendor::Microsoft);
    }

    #[test]
    fn infer_vendor_unknown_id() {
        assert_eq!(infer_gpu_vendor(Some(0xffff)), GpuVendor::Unknown);
        assert_eq!(infer_gpu_vendor(Some(0x0000)), GpuVendor::Unknown);
        assert_eq!(infer_gpu_vendor(None), GpuVendor::Unknown);
    }

    #[test]
    fn inventory_trait_is_object_safe_enough_for_basic_use() {
        let inventory = TestInventory;
        assert!(inventory.list_gpus().unwrap().is_empty());
    }
}
