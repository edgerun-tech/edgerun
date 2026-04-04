use lifegraph_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcPowerState {
    Unknown,
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcDeviceInfo {
    pub provider: String,
    pub device_name: String,
    pub power_state: NfcPowerState,
    pub protocol_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NfcTargetObservation {
    pub target_id: String,
    pub protocol_name: Option<String>,
    pub technology: Option<String>,
    pub observed_at_unix_ms: i64,
}

pub trait NfcDevice: CapabilityProvider {
    fn device_info(&self) -> Result<NfcDeviceInfo, lifegraph_capabilities::CapabilityError>;
    fn power_state(&self) -> Result<NfcPowerState, lifegraph_capabilities::CapabilityError>;
}

pub fn default_nfc_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::Radio, CapabilityEventKind::State],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Observe,
            CapabilityOperation::Control,
        ],
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nfc_descriptor_kind_is_correct() {
        let descriptor = default_nfc_descriptor("nfc", "nfc0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Radio as i32)));
    }
}
