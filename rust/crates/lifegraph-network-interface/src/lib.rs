use lifegraph_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkInterfaceKind {
    Unknown,
    Ethernet,
    Wireless,
    Loopback,
    Bridge,
    Tunnel,
    Vlan,
    Virtual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAdminState {
    Unknown,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkLinkState {
    Unknown,
    Up,
    Down,
    Dormant,
    LowerLayerDown,
    NotPresent,
    Testing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkInterfaceInfo {
    pub provider: String,
    pub interface_name: String,
    pub kind: NetworkInterfaceKind,
    pub mac_address: Option<String>,
    pub mtu: Option<u32>,
    pub admin_state: NetworkAdminState,
    pub link_state: NetworkLinkState,
}

pub trait NetworkInterfaceController: CapabilityProvider {
    fn interface_info(
        &self,
    ) -> Result<NetworkInterfaceInfo, lifegraph_capabilities::CapabilityError>;
    fn set_admin_state(
        &self,
        state: NetworkAdminState,
    ) -> Result<NetworkAdminState, lifegraph_capabilities::CapabilityError>;
}

pub fn default_network_interface_descriptor(
    provider: &str,
    instance_id: &str,
) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio, CapabilityModality::Text],
        &[CapabilityEventKind::State],
        &[CapabilityOperation::Query, CapabilityOperation::Control],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netif_descriptor_is_communication_stateful() {
        let descriptor = default_network_interface_descriptor("linux-netif", "eth0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::State as i32)));
    }
}
