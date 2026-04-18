use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};
use edgerun_quectel_ec200a::DtaNetwork;

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
