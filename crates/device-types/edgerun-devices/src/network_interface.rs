extern crate alloc;
mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

use crate::quectel_ec200a::DtaNetwork;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole, capability_descriptor,
};
use prelude::v1::*;

/// Network interface supported by Quectel DTA modem
pub struct DtaNetworkInterface {
    enabled: bool,
    apn: String,
}

impl DtaNetworkInterface {
    pub fn new(enabled: bool, apn: String) -> Self {
        Self { enabled, apn }
    }
}

impl CapabilityProvider for DtaNetworkInterface {
    fn descriptor(&self) -> CapabilityDescriptor {
        capability_descriptor(
            "quectel-ec200a-dta",
            "dta-network",
            CapabilityRole::Communication,
            &[CapabilityModality::Radio, CapabilityModality::Text],
            &[CapabilityEventKind::State],
            &[CapabilityOperation::Query, CapabilityOperation::Control],
            Vec::new(),
        )
    }
}

/// Network interface kinds
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkInterfaceKind {
    Ethernet,
    Wireless,
    Loopback,
    Bridge,
    Virtual,
    Vlan,
    Tunnel,
    Unknown,
}

/// Network link states
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkLinkState {
    Up,
    Down,
    Dormant,
    LowerLayerDown,
    NotPresent,
    Testing,
    Unknown,
}

/// Network admin states
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAdminState {
    Up,
    Down,
    Unknown,
}

/// Network interface information
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

/// Network interface controller trait
pub trait NetworkInterfaceController: CapabilityProvider {
    fn interface_info(&self) -> Result<NetworkInterfaceInfo, CapabilityError>;
    fn set_admin_state(
        &self,
        state: NetworkAdminState,
    ) -> Result<NetworkAdminState, CapabilityError>;
}

/// Create a default network interface descriptor
pub fn default_network_interface_descriptor(
    provider: &str,
    interface_name: &str,
) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        interface_name,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::State],
        &[CapabilityOperation::Query, CapabilityOperation::Control],
        Vec::new(),
    )
}
