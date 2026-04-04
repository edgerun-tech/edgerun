use lifegraph_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CecLogicalAddress {
    Tv = 0,
    Playback1 = 4,
    AudioSystem = 5,
    Playback2 = 8,
    Playback3 = 11,
    Unregistered = 15,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CecDrmConnectorInfo {
    pub card_no: u32,
    pub connector_id: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CecCapabilities {
    pub can_set_physical_address: bool,
    pub can_set_logical_addresses: bool,
    pub can_transmit: bool,
    pub passthrough: bool,
    pub remote_control: bool,
    pub monitor_all: bool,
    pub needs_hpd: bool,
    pub monitor_pin: bool,
    pub connector_info: bool,
    pub reply_vendor_id: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CecAdapterInfo {
    pub provider: String,
    pub instance_id: String,
    pub adapter_name: String,
    pub driver_name: Option<String>,
    pub device_node: Option<String>,
    pub physical_address: Option<u16>,
    pub logical_address_mask: Option<u16>,
    pub available_log_addrs: Option<u32>,
    pub capabilities: CecCapabilities,
    pub drm_connector: Option<CecDrmConnectorInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CecMessage {
    pub bytes: Vec<u8>,
}

pub trait CecAdapterDevice: CapabilityProvider {
    fn adapter_info(&self) -> Result<CecAdapterInfo, CapabilityError>;
    fn transmit(&mut self, message: &CecMessage) -> Result<(), CapabilityError>;
}

pub fn default_cec_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Display, CapabilityModality::Text],
        &[CapabilityEventKind::State, CapabilityEventKind::Display],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Control,
            CapabilityOperation::Invoke,
        ],
        vec![constraint(CapabilityConstraintKind::RequireLocalOnly)],
    )
}

pub fn cec_header(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> u8 {
    ((initiator as u8) << 4) | destination as u8
}

pub fn image_view_on(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> CecMessage {
    CecMessage {
        bytes: vec![cec_header(initiator, destination), 0x04],
    }
}

pub fn text_view_on(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> CecMessage {
    CecMessage {
        bytes: vec![cec_header(initiator, destination), 0x0d],
    }
}

pub fn standby(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> CecMessage {
    CecMessage {
        bytes: vec![cec_header(initiator, destination), 0x36],
    }
}

pub fn active_source(initiator: CecLogicalAddress, physical_address: u16) -> CecMessage {
    CecMessage {
        bytes: vec![
            cec_header(initiator, CecLogicalAddress::Unregistered),
            0x82,
            (physical_address >> 8) as u8,
            physical_address as u8,
        ],
    }
}

pub fn set_stream_path(initiator: CecLogicalAddress, physical_address: u16) -> CecMessage {
    CecMessage {
        bytes: vec![
            cec_header(initiator, CecLogicalAddress::Unregistered),
            0x86,
            (physical_address >> 8) as u8,
            physical_address as u8,
        ],
    }
}

pub fn wake_sequence(
    initiator: CecLogicalAddress,
    physical_address: Option<u16>,
) -> Vec<CecMessage> {
    let mut messages = vec![image_view_on(initiator, CecLogicalAddress::Tv)];
    if let Some(physical_address) = physical_address {
        messages.push(active_source(initiator, physical_address));
        messages.push(set_stream_path(initiator, physical_address));
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;

    impl CapabilityProvider for Dummy {
        fn descriptor(&self) -> CapabilityDescriptor {
            default_cec_descriptor("linux-cec", "cec0")
        }
    }

    impl CecAdapterDevice for Dummy {
        fn adapter_info(&self) -> Result<CecAdapterInfo, CapabilityError> {
            Ok(CecAdapterInfo {
                provider: "test".into(),
                instance_id: "cec0".into(),
                adapter_name: "cec0".into(),
                driver_name: None,
                device_node: None,
                physical_address: None,
                logical_address_mask: None,
                available_log_addrs: None,
                capabilities: CecCapabilities::default(),
                drm_connector: None,
            })
        }

        fn transmit(&mut self, _message: &CecMessage) -> Result<(), CapabilityError> {
            Ok(())
        }
    }

    #[test]
    fn descriptor_is_local_communication_capability() {
        let descriptor = default_cec_descriptor("linux-cec", "cec0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireLocalOnly as i32));
    }

    #[test]
    fn wake_sequence_builds_expected_messages() {
        let sequence = wake_sequence(CecLogicalAddress::Playback1, Some(0x2100));
        assert_eq!(sequence.len(), 3);
        assert_eq!(sequence[0].bytes, vec![0x40, 0x04]);
        assert_eq!(sequence[1].bytes, vec![0x4f, 0x82, 0x21, 0x00]);
        assert_eq!(sequence[2].bytes, vec![0x4f, 0x86, 0x21, 0x00]);
    }

    #[test]
    fn standby_message_targets_tv() {
        let message = standby(CecLogicalAddress::Playback1, CecLogicalAddress::Tv);
        assert_eq!(message.bytes, vec![0x40, 0x36]);
    }

    #[test]
    fn trait_round_trip_compiles() {
        let mut dummy = Dummy;
        dummy
            .transmit(&image_view_on(
                CecLogicalAddress::Playback1,
                CecLogicalAddress::Tv,
            ))
            .unwrap();
        assert_eq!(dummy.adapter_info().unwrap().instance_id, "cec0");
    }
}
