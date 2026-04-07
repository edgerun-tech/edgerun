use edgerun_capabilities::{
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
    ) -> Result<NetworkInterfaceInfo, edgerun_capabilities::CapabilityError>;
    fn set_admin_state(
        &self,
        state: NetworkAdminState,
    ) -> Result<NetworkAdminState, edgerun_capabilities::CapabilityError>;
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

    // --- NetworkInterfaceKind ---

    #[test]
    fn kind_all_variants_exist() {
        let kinds = [
            NetworkInterfaceKind::Unknown,
            NetworkInterfaceKind::Ethernet,
            NetworkInterfaceKind::Wireless,
            NetworkInterfaceKind::Loopback,
            NetworkInterfaceKind::Bridge,
            NetworkInterfaceKind::Tunnel,
            NetworkInterfaceKind::Vlan,
            NetworkInterfaceKind::Virtual,
        ];
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b, "kind {i} should differ from kind {j}");
                }
            }
        }
    }

    #[test]
    fn kind_debug_format() {
        assert_eq!(format!("{:?}", NetworkInterfaceKind::Ethernet), "Ethernet");
        assert_eq!(format!("{:?}", NetworkInterfaceKind::Wireless), "Wireless");
        assert_eq!(format!("{:?}", NetworkInterfaceKind::Loopback), "Loopback");
        assert_eq!(format!("{:?}", NetworkInterfaceKind::Virtual), "Virtual");
    }

    #[test]
    fn kind_is_copy_and_clone() {
        let k = NetworkInterfaceKind::Bridge;
        assert_eq!(k.clone(), k);
    }

    // --- NetworkAdminState ---

    #[test]
    fn admin_state_variants_are_distinct() {
        assert_ne!(NetworkAdminState::Unknown, NetworkAdminState::Up);
        assert_ne!(NetworkAdminState::Unknown, NetworkAdminState::Down);
        assert_ne!(NetworkAdminState::Up, NetworkAdminState::Down);
    }

    #[test]
    fn admin_state_debug() {
        assert_eq!(format!("{:?}", NetworkAdminState::Up), "Up");
        assert_eq!(format!("{:?}", NetworkAdminState::Down), "Down");
    }

    // --- NetworkLinkState ---

    #[test]
    fn link_state_all_variants_exist() {
        let states = [
            NetworkLinkState::Unknown,
            NetworkLinkState::Up,
            NetworkLinkState::Down,
            NetworkLinkState::Dormant,
            NetworkLinkState::LowerLayerDown,
            NetworkLinkState::NotPresent,
            NetworkLinkState::Testing,
        ];
        for (i, a) in states.iter().enumerate() {
            for (j, b) in states.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b);
                }
            }
        }
    }

    #[test]
    fn link_state_debug() {
        assert_eq!(format!("{:?}", NetworkLinkState::Dormant), "Dormant");
        assert_eq!(
            format!("{:?}", NetworkLinkState::LowerLayerDown),
            "LowerLayerDown"
        );
    }

    #[test]
    fn link_state_is_copy_and_clone() {
        let s = NetworkLinkState::NotPresent;
        assert_eq!(s.clone(), s);
    }

    // --- NetworkInterfaceInfo ---

    #[test]
    fn interface_info_full() {
        let info = NetworkInterfaceInfo {
            provider: "linux-netif".to_string(),
            interface_name: "eth0".to_string(),
            kind: NetworkInterfaceKind::Ethernet,
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            mtu: Some(1500),
            admin_state: NetworkAdminState::Up,
            link_state: NetworkLinkState::Up,
        };
        assert_eq!(info.interface_name, "eth0");
        assert_eq!(info.kind, NetworkInterfaceKind::Ethernet);
        assert_eq!(info.mtu, Some(1500));
    }

    #[test]
    fn interface_info_minimized() {
        let info = NetworkInterfaceInfo {
            provider: "test".to_string(),
            interface_name: "lo".to_string(),
            kind: NetworkInterfaceKind::Loopback,
            mac_address: None,
            mtu: None,
            admin_state: NetworkAdminState::Unknown,
            link_state: NetworkLinkState::Unknown,
        };
        assert!(info.mac_address.is_none());
        assert!(info.mtu.is_none());
    }

    #[test]
    fn interface_info_clone_and_eq() {
        let info = NetworkInterfaceInfo {
            provider: "p".to_string(),
            interface_name: "i".to_string(),
            kind: NetworkInterfaceKind::Unknown,
            mac_address: None,
            mtu: None,
            admin_state: NetworkAdminState::Unknown,
            link_state: NetworkLinkState::Unknown,
        };
        assert_eq!(info.clone(), info);
    }
}
