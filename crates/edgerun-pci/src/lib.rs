use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PciDeviceInfo {
    pub provider: String,
    pub address: String,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,
    pub subsystem_vendor_id: Option<u16>,
    pub subsystem_device_id: Option<u16>,
    pub class_code: Option<u32>,
    pub revision: Option<u8>,
    pub driver: Option<String>,
    pub numa_node: Option<i32>,
    pub current_link_speed: Option<String>,
    pub current_link_width: Option<u32>,
    pub max_link_speed: Option<String>,
    pub max_link_width: Option<u32>,
    pub parent_address: Option<String>,
    pub child_addresses: Vec<String>,
}

pub trait PciInventory: CapabilityProvider {
    fn list_devices(&self) -> Result<Vec<PciDeviceInfo>, edgerun_capabilities::CapabilityError>;
}

pub fn default_pci_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Text],
        &[CapabilityEventKind::State],
        &[CapabilityOperation::Query, CapabilityOperation::Observe],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PciDeviceInfo ---

    #[test]
    fn device_info_full() {
        let info = PciDeviceInfo {
            provider: "linux-pci".to_string(),
            address: "0000:01:00.0".to_string(),
            vendor_id: Some(0x144d),
            device_id: Some(0xa80a),
            subsystem_vendor_id: Some(0x144d),
            subsystem_device_id: Some(0xa801),
            class_code: Some(0x010802),
            revision: Some(0x01),
            driver: Some("nvme".to_string()),
            numa_node: Some(0),
            current_link_speed: Some("8 GT/s".to_string()),
            current_link_width: Some(4),
            max_link_speed: Some("8 GT/s".to_string()),
            max_link_width: Some(4),
            parent_address: None,
            child_addresses: vec![],
        };
        assert_eq!(info.address, "0000:01:00.0");
        assert_eq!(info.vendor_id, Some(0x144d));
        assert_eq!(info.class_code, Some(0x010802));
        assert!(info.child_addresses.is_empty());
    }

    #[test]
    fn device_info_minimal() {
        let info = PciDeviceInfo {
            provider: "test".to_string(),
            address: "0000:00:00.0".to_string(),
            vendor_id: None,
            device_id: None,
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            class_code: None,
            revision: None,
            driver: None,
            numa_node: None,
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            parent_address: None,
            child_addresses: vec![],
        };
        assert!(info.vendor_id.is_none());
        assert!(info.driver.is_none());
        assert!(info.numa_node.is_none());
    }

    #[test]
    fn device_info_with_parent_and_children() {
        let info = PciDeviceInfo {
            provider: "p".to_string(),
            address: "0000:02:00.0".to_string(),
            vendor_id: None,
            device_id: None,
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            class_code: None,
            revision: None,
            driver: None,
            numa_node: Some(-1),
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            parent_address: Some("0000:00:01.0".to_string()),
            child_addresses: vec!["0000:03:00.0".to_string()],
        };
        assert_eq!(info.parent_address.as_deref(), Some("0000:00:01.0"));
        assert_eq!(info.child_addresses, vec!["0000:03:00.0"]);
        assert_eq!(info.numa_node, Some(-1));
    }

    #[test]
    fn device_info_clone() {
        let info = PciDeviceInfo {
            provider: "p".to_string(),
            address: "a".to_string(),
            vendor_id: None,
            device_id: None,
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            class_code: None,
            revision: None,
            driver: None,
            numa_node: None,
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            parent_address: None,
            child_addresses: vec![],
        };
        assert_eq!(info.clone(), info);
    }

    #[test]
    fn device_info_debug() {
        let info = PciDeviceInfo {
            provider: "linux-pci".to_string(),
            address: "0000:01:00.0".to_string(),
            vendor_id: None,
            device_id: None,
            subsystem_vendor_id: None,
            subsystem_device_id: None,
            class_code: None,
            revision: None,
            driver: None,
            numa_node: None,
            current_link_speed: None,
            current_link_width: None,
            max_link_speed: None,
            max_link_width: None,
            parent_address: None,
            child_addresses: vec![],
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("PciDeviceInfo"));
        assert!(debug.contains("0000:01:00.0"));
    }

    // --- default_pci_descriptor ---

    #[test]
    fn pci_descriptor_is_stateful() {
        let descriptor = default_pci_descriptor("linux-pci", "root");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }

    #[test]
    fn pci_descriptor_has_query_and_observe() {
        let descriptor = default_pci_descriptor("linux-pci", "0000:01:00.0");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
    }

    #[test]
    fn pci_descriptor_has_text_modality() {
        let descriptor = default_pci_descriptor("linux-pci", "root");
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Text as i32]
        );
    }

    #[test]
    fn pci_descriptor_uses_instance_id() {
        let descriptor = default_pci_descriptor("my-pci", "0000:02:00.0");
        assert_eq!(descriptor.provider_name, "my-pci");
        assert_eq!(descriptor.provider_instance_id, "0000:02:00.0");
    }
}
