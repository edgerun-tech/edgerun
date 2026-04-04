use lifegraph_capabilities::{
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
    fn list_devices(&self) -> Result<Vec<UsbDeviceInfo>, lifegraph_capabilities::CapabilityError>;
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

    #[test]
    fn usb_descriptor_is_stateful() {
        let descriptor = default_usb_descriptor("linux-usb", "root");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }
}
