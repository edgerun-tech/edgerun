use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbSpeed {
    Unknown,
    Low,
    Full,
    High,
    Super,
    SuperPlus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbInterfaceInfo {
    pub instance_id: String,
    pub interface_number: Option<u8>,
    pub alternate_setting: Option<u8>,
    pub interface_class: Option<u8>,
    pub interface_subclass: Option<u8>,
    pub interface_protocol: Option<u8>,
    pub interface_name: Option<String>,
    pub driver: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbDeviceInfo {
    pub provider: String,
    pub instance_id: String,
    pub bus_num: Option<u32>,
    pub dev_num: Option<u32>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
    pub usb_version: Option<String>,
    pub configuration_value: Option<u8>,
    pub configuration_name: Option<String>,
    pub port_path: Option<String>,
    pub max_children: Option<u32>,
    pub speed: UsbSpeed,
    pub driver: Option<String>,
    pub parent_instance_id: Option<String>,
    pub child_instance_ids: Vec<String>,
    pub interfaces: Vec<UsbInterfaceInfo>,
}

pub trait UsbInventory: CapabilityProvider {
    fn list_devices(&self) -> Result<Vec<UsbDeviceInfo>, edgerun_capabilities::CapabilityError>;
}

pub fn default_usb_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
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

    // --- UsbSpeed ---

    #[test]
    fn speed_all_variants() {
        let speeds = [
            UsbSpeed::Unknown,
            UsbSpeed::Low,
            UsbSpeed::Full,
            UsbSpeed::High,
            UsbSpeed::Super,
            UsbSpeed::SuperPlus,
        ];
        for (i, a) in speeds.iter().enumerate() {
            for (j, b) in speeds.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b);
                }
            }
        }
    }

    #[test]
    fn speed_debug() {
        assert_eq!(format!("{:?}", UsbSpeed::Low), "Low");
        assert_eq!(format!("{:?}", UsbSpeed::Full), "Full");
        assert_eq!(format!("{:?}", UsbSpeed::SuperPlus), "SuperPlus");
    }

    #[test]
    fn speed_is_copy_and_clone() {
        let s = UsbSpeed::High;
        assert_eq!(s.clone(), s);
    }

    // --- UsbInterfaceInfo ---

    #[test]
    fn interface_info_full() {
        let info = UsbInterfaceInfo {
            instance_id: "1-2:1.0".to_string(),
            interface_number: Some(0),
            alternate_setting: Some(0),
            interface_class: Some(0xff),
            interface_subclass: Some(0x00),
            interface_protocol: Some(0x00),
            interface_name: Some("Vendor".to_string()),
            driver: Some("usbfs".to_string()),
        };
        assert_eq!(info.instance_id, "1-2:1.0");
        assert_eq!(info.interface_class, Some(0xff));
    }

    #[test]
    fn interface_info_all_none() {
        let info = UsbInterfaceInfo {
            instance_id: "x".to_string(),
            interface_number: None,
            alternate_setting: None,
            interface_class: None,
            interface_subclass: None,
            interface_protocol: None,
            interface_name: None,
            driver: None,
        };
        assert!(info.interface_number.is_none());
        assert!(info.driver.is_none());
    }

    #[test]
    fn interface_info_clone() {
        let info = UsbInterfaceInfo {
            instance_id: "i".to_string(),
            interface_number: None,
            alternate_setting: None,
            interface_class: None,
            interface_subclass: None,
            interface_protocol: None,
            interface_name: None,
            driver: None,
        };
        assert_eq!(info.clone(), info);
    }

    // --- UsbDeviceInfo ---

    #[test]
    fn device_info_construction() {
        let info = UsbDeviceInfo {
            provider: "linux-usb".to_string(),
            instance_id: "1-2".to_string(),
            bus_num: Some(1),
            dev_num: Some(3),
            vendor_id: Some(0x27c6),
            product_id: Some(0x609c),
            manufacturer: Some("Goodix".to_string()),
            product_name: Some("Fingerprint".to_string()),
            serial_number: None,
            usb_version: Some("2.00".to_string()),
            configuration_value: Some(1),
            configuration_name: Some("Default".to_string()),
            port_path: Some("2".to_string()),
            max_children: Some(0),
            speed: UsbSpeed::Full,
            driver: None,
            parent_instance_id: None,
            child_instance_ids: vec![],
            interfaces: vec![],
        };
        assert_eq!(info.vendor_id, Some(0x27c6));
        assert_eq!(info.speed, UsbSpeed::Full);
        assert!(info.child_instance_ids.is_empty());
    }

    #[test]
    fn device_info_with_children() {
        let info = UsbDeviceInfo {
            provider: "p".to_string(),
            instance_id: "1-4".to_string(),
            bus_num: None,
            dev_num: None,
            vendor_id: None,
            product_id: None,
            manufacturer: None,
            product_name: None,
            serial_number: None,
            usb_version: None,
            configuration_value: None,
            configuration_name: None,
            port_path: None,
            max_children: None,
            speed: UsbSpeed::Unknown,
            driver: None,
            parent_instance_id: None,
            child_instance_ids: vec!["1-4.1".to_string(), "1-4.2".to_string()],
            interfaces: vec![],
        };
        assert_eq!(info.child_instance_ids.len(), 2);
    }

    #[test]
    fn device_info_clone() {
        let info = UsbDeviceInfo {
            provider: "p".to_string(),
            instance_id: "i".to_string(),
            bus_num: None,
            dev_num: None,
            vendor_id: None,
            product_id: None,
            manufacturer: None,
            product_name: None,
            serial_number: None,
            usb_version: None,
            configuration_value: None,
            configuration_name: None,
            port_path: None,
            max_children: None,
            speed: UsbSpeed::Unknown,
            driver: None,
            parent_instance_id: None,
            child_instance_ids: vec![],
            interfaces: vec![],
        };
        assert_eq!(info.clone(), info);
    }

    // --- default_usb_descriptor ---

    #[test]
    fn usb_descriptor_is_stateful() {
        let descriptor = default_usb_descriptor("linux-usb", "root");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }

    #[test]
    fn usb_descriptor_has_query_and_observe() {
        let descriptor = default_usb_descriptor("linux-usb", "1-2");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Observe as i32)));
    }

    #[test]
    fn usb_descriptor_has_text_modality() {
        let descriptor = default_usb_descriptor("linux-usb", "root");
        assert_eq!(descriptor.modalities, vec![CapabilityModality::Text as i32]);
    }
}
